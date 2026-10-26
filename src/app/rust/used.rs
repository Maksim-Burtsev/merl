use std::sync::LazyLock;

use super::{
    App, CrateSrc, Hit, InCrate, Kind, MacroUse, Path, PathBuf, Regex, Typed, bound, declares_mod,
    search,
};

impl App {
    pub(super) fn rust_used_type(
        &self,
        file: &Path,
        text: &str,
        name: &str,
        inline_mod: Option<usize>,
        line0: usize,
    ) -> Option<Typed> {
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Rust, text);
        let around = search::rust_around(&lines, line0, &literal);
        let in_scope = |u: &search::RustUse| {
            inline_mod.is_none_or(|m| {
                let at = text
                    .get(..u.start_byte)
                    .map_or(0, |t| t.matches('\n').count());
                search::rust_around(&lines, at, &literal)
                    .first()
                    .is_some_and(|p| *p >= m && around.contains(p))
            })
        };
        let mut bound = search::rust_uses(text)
            .into_iter()
            .filter(|u| u.name == name && in_scope(u));
        let (Some(used), None) = (bound.next(), bound.next()) else {
            return None;
        };
        let (root, rest) = match used.path.split_first() {
            _ if inline_mod.is_none() && !used.in_column_zero => return None,
            Some((root, rest)) if inline_mod.is_some() => match root.as_str() {
                "crate" => ("crate", rest),
                "self" | "super" => return None,
                _ => (root.as_str(), rest),
            },
            Some((root, rest)) if matches!(root.as_str(), "crate" | "super" | "self") => {
                (root.as_str(), rest)
            }
            Some((root, _)) if declares_mod(text, root) => ("self", used.path.as_slice()),
            Some((root, rest)) if root != "self" => (root.as_str(), rest),
            _ => return None,
        };
        let (krate, module, rest) = match root {
            "crate" | "super" | "self" => {
                let (krate, module) = self.rust_own_crate(file, root, rest)?;
                (krate, module, rest)
            }
            _ => {
                let krate = self
                    .workspace_crate(root)
                    .or_else(|| self.locked_registry_crate(root))?;
                match self.rust_crate_reexported(&krate, rest) {
                    Some(inner) => (inner, Vec::new(), rest.get(1..)?),
                    None => (krate, Vec::new(), rest),
                }
            }
        };
        self.rust_type_in(&krate, &module, rest)
    }

    pub(super) fn rust_registry_path(
        &self,
        text: &str,
        path: &[String],
        name: &str,
    ) -> Option<Typed> {
        let (root, rest) = path.split_first()?;
        if bound(&search::imports(Kind::Rust, text), root).is_some() || declares_mod(text, root) {
            return None;
        }
        let krate = self.locked_registry_crate(root)?;
        let mut rest = rest.to_vec();
        rest.push(name.to_owned());
        self.rust_type_in(&krate, &[], &rest)
    }

    pub(super) fn rust_registry_elsewhere(
        &self,
        file: &Path,
        text: &str,
        name: &str,
    ) -> Option<Typed> {
        static GLOB: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+(?:::)?(\w+)[^;]*\*[^;]*;")
                .expect("the regex is a literal")
        });
        let home = self.registry_home(file)?;
        let ours = |root: &str| matches!(root, "crate" | "self" | "super");
        let mut used = search::rust_uses(text)
            .into_iter()
            .filter(|u| u.name == name);
        match (used.next(), used.next()) {
            (Some(u), None) => {
                let (root, rest) = match u.path.split_first()? {
                    (root, rest) if ours(root) => (root.as_str(), rest),
                    (root, _) if declares_mod(text, root) => ("self", u.path.as_slice()),
                    _ => return None,
                };
                if !u.in_column_zero {
                    return None;
                }
                let mut module: Vec<String> = file
                    .strip_prefix(&home.src)
                    .ok()?
                    .with_extension("")
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                if matches!(module.as_slice(), [m] if m == "lib" || m == "main")
                    || module.last().is_some_and(|m| m == "mod")
                {
                    module.pop();
                }
                match root {
                    "crate" => module.clear(),
                    "super" => {
                        let supers = 1 + rest.iter().take_while(|p| *p == "super").count();
                        module.truncate(module.len().checked_sub(supers)?);
                    }
                    _ => {}
                }
                self.rust_type_in(&home, &module, rest)
            }
            (None, _) => {
                let roots: Vec<&str> = GLOB
                    .captures_iter(text)
                    .filter_map(|c| c.get(1))
                    .map(|m| m.as_str())
                    .collect();
                if roots.is_empty() || !roots.iter().all(|r| ours(r)) {
                    return None;
                }
                let decls: Vec<Hit> = self
                    .grep_in(&search::rust_type_decl_pattern(name), &home.files)
                    .into_iter()
                    .filter(|h| {
                        self.text_of(&h.path).is_some_and(|t| {
                            search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) != Some(&true)
                        })
                    })
                    .collect();
                let [hit] = <[Hit; 1]>::try_from(decls).ok()?;
                Some(Typed {
                    name: name.to_owned(),
                    path: hit.path,
                    line: hit.line1,
                })
            }
            _ => None,
        }
    }

    fn rust_type_in(&self, krate: &CrateSrc, module: &[String], rest: &[String]) -> Option<Typed> {
        static DECL: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|union)\s+([A-Za-z_]\w*)")
                .expect("the regex is a literal")
        });
        let rest: Vec<String> = rest.iter().filter(|p| *p != "super").cloned().collect();
        let InCrate {
            hits, proven: true, ..
        } = self.rust_in_crate(krate, module, &rest, MacroUse::NotMacro, 0)?
        else {
            return None;
        };
        let types: Vec<(Hit, String)> = hits
            .into_iter()
            .filter_map(|h| {
                let real = DECL.captures(&h.text)?[1].to_owned();
                Some((h, real))
            })
            .collect();
        let [(hit, real)] = <[(Hit, String); 1]>::try_from(types).ok()?;
        Some(Typed {
            name: real,
            path: hit.path,
            line: hit.line1,
        })
    }

    pub(super) fn rust_alias(&self, name: &str) -> Option<Typed> {
        let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
        let pattern = format!(
            r"^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+{}\s*=",
            regex::escape(name)
        );
        let aliases: Vec<Hit> = self
            .grep(&pattern, false, false, rs)
            .unwrap_or_default()
            .into_iter()
            .filter(|h| {
                self.text_of(&h.path).is_some_and(|t| {
                    search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) != Some(&true)
                })
            })
            .collect();
        let [hit] = <[Hit; 1]>::try_from(aliases).ok()?;
        let target = hit
            .text
            .split_once('=')?
            .1
            .trim()
            .trim_end_matches(';')
            .trim();
        let real = search::rust_type_name(target)?.name;
        let declared = self
            .grep(&search::rust_type_decl_pattern(&real), false, false, rs)
            .unwrap_or_default();
        if real == name || declared.is_empty() {
            return None;
        }
        self.rust_resolve(&hit.path, target, hit.line1 - 1)
    }

    fn rust_crate_reexported(&self, krate: &CrateSrc, rest: &[String]) -> Option<CrateSrc> {
        let [alias, _, ..] = rest else {
            return None;
        };
        let lib = krate.src.join("lib.rs");
        if !krate.files.contains(&lib) {
            return None;
        }
        let text = self.text_of(&lib)?;
        let re = Regex::new(&format!(
            r"(?m)^\s*pub\s+(?:extern\s+crate|use)\s+(?:::)?(\w+)\s+as\s+{}\s*;",
            regex::escape(alias)
        ))
        .ok()?;
        let target = re.captures(&text)?[1].to_owned();
        if matches!(target.as_str(), "crate" | "self" | "super") || declares_mod(&text, &target) {
            return None;
        }
        self.workspace_crate(&target)
    }
}

pub(super) fn registry_src(r: &Path, all: &[PathBuf], name: &str) -> Option<CrateSrc> {
    let dir = r.file_name()?.to_string_lossy().into_owned();
    let (i, _) = dir.rmatch_indices('-').find(|(i, _)| {
        dir.get(i + 1..)
            .is_some_and(|v| v.starts_with(|c: char| c.is_ascii_digit()))
    })?;
    if dir.get(..i)?.replace('-', "_") != name {
        return None;
    }
    let src = r.join("src");
    let files: Vec<PathBuf> = all
        .iter()
        .filter(|f| f.starts_with(&src))
        .cloned()
        .collect();
    (!files.is_empty()).then_some(CrateSrc { src, files })
}

impl App {
    pub(super) fn locked_registry_crate(&self, name: &str) -> Option<CrateSrc> {
        let (roots, all) = self.external.get(&Kind::Rust)?;
        let key = regex::escape(name).replace('_', "[-_]");
        let renamed = Regex::new(&format!(
            r"(?m)^\s*(?:{key}\s*=[^\n]*\bpackage\s*=|\[[^\]\n]*dependencies\.{key}\][^\[]*\bpackage\s*=)"
        ))
        .ok()?;
        if self.files.iter().any(|f| {
            f.file_name().is_some_and(|n| n == "Cargo.toml")
                && self.text_of(f).is_some_and(|t| renamed.is_match(&t))
        }) {
            return None;
        }
        let mut found = roots.iter().filter_map(|r| registry_src(r, all, name));
        match (found.next(), found.next()) {
            (Some(one), None) => Some(one),
            _ => None,
        }
    }

    pub(super) fn registry_home(&self, file: &Path) -> Option<CrateSrc> {
        let (roots, all) = self.external.get(&Kind::Rust)?;
        let src = roots
            .iter()
            .map(|r| r.join("src"))
            .find(|s| file.starts_with(s))?;
        let files = all
            .iter()
            .filter(|f| f.starts_with(&src))
            .cloned()
            .collect();
        Some(CrateSrc { src, files })
    }
}
