use std::sync::LazyLock;

use super::{
    App, Candidate, CrateSrc, Hit, InCrate, Kind, MacroUse, Path, PathBuf, Reason, Regex, Typed,
    bound, declares_mod, search,
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
        line0: usize,
    ) -> Option<Typed> {
        let home = self.registry_home(file)?;
        let ours = |root: &str| matches!(root, "crate" | "self" | "super");
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
                let lines: Vec<&str> = text.lines().collect();
                if !GLOB.is_match(text)
                    || search::rust_inline_mod(&lines, line0).is_some()
                    || !self.rust_globs_lack(&home, &module, text, name, &mut Vec::new())
                {
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

    fn rust_globs_lack(
        &self,
        home: &CrateSrc,
        module: &[String],
        text: &str,
        name: &str,
        seen: &mut Vec<Vec<String>>,
    ) -> bool {
        GLOB.captures_iter(text).all(|c| {
            let Some(target) = glob_module(module, &c[1]) else {
                return false;
            };
            if seen.contains(&target) {
                return true;
            }
            seen.push(target.clone());
            let file = if target.is_empty() {
                vec![home.src.join("lib.rs"), home.src.join("main.rs")]
            } else {
                let at = target.iter().fold(home.src.clone(), |p, m| p.join(m));
                vec![at.with_extension("rs"), at.join("mod.rs")]
            }
            .into_iter()
            .find(|f| home.files.contains(f));
            let Some(globbed) = file.and_then(|f| self.text_of(&f)) else {
                return false;
            };
            !search::rust_uses(&globbed).iter().any(|u| u.name == name)
                && self.rust_globs_lack(home, &target, &globbed, name, seen)
        })
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

static GLOB: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^(?:pub(?:\([^)]*\))?\s+)?use\s+([^;]*\*[^;]*);")
        .expect("the regex is a literal")
});

fn glob_module(module: &[String], written: &str) -> Option<Vec<String>> {
    let written: String = written.split_whitespace().collect();
    let parts: Vec<&str> = written
        .trim_start_matches("::")
        .strip_suffix("::*")?
        .split("::")
        .collect();
    if !parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_alphanumeric() || c == '_'))
    {
        return None;
    }
    let supers = parts.iter().take_while(|p| **p == "super").count();
    let (mut at, rest) = match *parts.first()? {
        "crate" => (Vec::new(), parts.get(1..)?),
        "self" => (module.to_vec(), parts.get(1..)?),
        "super" => (
            module.get(..module.len().checked_sub(supers)?)?.to_vec(),
            parts.get(supers..)?,
        ),
        _ => return None,
    };
    if rest
        .iter()
        .any(|p| matches!(*p, "crate" | "self" | "super"))
    {
        return None;
    }
    at.extend(rest.iter().map(|p| (*p).to_owned()));
    Some(at)
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

    pub(in crate::app) fn rust_glob_variant(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        imports: &[search::Import],
    ) -> Vec<Candidate> {
        let kind = Kind::Rust;
        let search::RustGlobUses {
            paths_before_star: globs,
            own_to_function: own,
        } = search::rust_glob_uses(text, self.line + 1);
        if globs.is_empty() {
            return Vec::new();
        }
        let top_level = || {
            let item = Regex::new(&search::def_patterns(kind, word)[..2].join("|"))
                .expect("an escaped name keeps the pattern valid");
            text.lines()
                .any(|l| !l.starts_with([' ', '\t']) && item.is_match(l))
        };
        if !own && (bound(imports, word).is_some() || top_level()) {
            return Vec::new();
        }
        let variants = self.rust_variants(here, word);
        let mut found = Vec::new();
        for glob in &globs {
            let Some(owner) = glob.last() else { continue };
            let enums = self
                .project_definitions(kind, here, owner, &search::def_patterns(kind, owner)[0])
                .into_iter()
                .filter(|h| h.text.contains("enum"))
                .collect::<Vec<_>>();
            let mine = enums.iter().any(|h| h.path == here);
            for v in &variants {
                let Some(t) = self.text_of(&v.path) else {
                    continue;
                };
                let full = format!("{owner}::{word}");
                let of = search::qualified(kind, &t, v.line1, word)
                    .is_some_and(|q| q == full || q.ends_with(&format!("::{full}")));
                let in_enum = of
                    && enums.iter().any(|e| {
                        e.path == v.path && (!mine || e.path == here) && e.line1 < v.line1
                    });
                if in_enum
                    && !found
                        .iter()
                        .any(|c: &Candidate| (&c.hit.path, c.hit.line1) == (&v.path, v.line1))
                {
                    found.push(Candidate {
                        hit: v.clone(),
                        reason: Reason::Import(format!("{}::*", glob.join("::"))),
                    });
                }
            }
        }
        if found.is_empty() && bound(imports, word).is_none() {
            for glob in &globs {
                found.extend(self.rust_dependency_variant(here, text, word, glob));
            }
        }
        found
    }

    fn rust_dependency_variant(
        &mut self,
        here: &Path,
        text: &str,
        word: &str,
        glob: &[String],
    ) -> Option<Candidate> {
        let (owner, path) = glob.split_last()?;
        if path
            .first()
            .is_some_and(|r| matches!(r.as_str(), "crate" | "self" | "super"))
        {
            return None;
        }
        let enums = self.rust_crate_path(here, text, owner, path, MacroUse::NotMacro)?;
        let [e] = enums.as_slice() else { return None };
        if matches!(e.reason, Reason::ByName) {
            return None;
        }
        let t = self.text_of(&e.hit.path)?;
        let lines: Vec<&str> = t.lines().collect();
        let indent = |l: &str| l.len() - l.trim_start().len();
        let decl = lines.get(e.hit.line1.checked_sub(1)?)?;
        if !decl.contains("enum ") {
            return None;
        }
        let variant = Regex::new(&search::rust_variant_pattern(word))
            .expect("an escaped name keeps the pattern valid");
        let k = (e.hit.line1..lines.len())
            .take_while(|&k| lines[k].trim().is_empty() || indent(lines[k]) > indent(decl))
            .find(|&k| variant.is_match(lines[k]) && search::rust_variant_at(&t, k + 1))?;
        Some(Candidate {
            hit: Hit {
                deleted: None,
                path: e.hit.path.clone(),
                line1: k + 1,
                byte_col: None,
                text: lines[k].to_owned(),
            },
            reason: Reason::Import(format!("{}::*", glob.join("::"))),
        })
    }
}
