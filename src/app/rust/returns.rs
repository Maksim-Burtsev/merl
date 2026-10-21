use std::collections::HashMap;
use std::sync::LazyLock;

use super::chain::Got;
use super::sysroot::DEREF;
use super::{App, Candidate, Hit, Kind, Path, PathBuf, Regex, rust_bare_type, search};

static MARKED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:&\s*(?:'\w+\s+)?(?:mut\s+)?)*(core|alloc|std)::([A-Z]\w*)\s*(?:<|$)")
        .expect("the regex is a literal")
});

static TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"'\w+|\b(?:[A-Za-z_]\w*::)*[A-Za-z_]\w*").expect("the regex is a literal")
});

fn top_level(args: &str) -> Vec<&str> {
    let mut depth = 0i32;
    let mut start = 0;
    let mut parts = Vec::new();
    for (i, c) in args.char_indices() {
        match c {
            '<' | '(' | '[' => depth += 1,
            '>' | ')' | ']' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(args.get(start..i).unwrap_or_default().trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(args.get(start..).unwrap_or_default().trim());
    parts.retain(|p| !p.is_empty());
    parts
}

fn angled(s: &str) -> Option<(&str, &str)> {
    let open = s.find('<')?;
    let mut depth = 0i32;
    for (i, c) in s.char_indices().skip_while(|&(i, _)| i < open) {
        match c {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return Some((s.get(open + 1..i)?, s.get(i + 1..)?));
                }
            }
            _ => {}
        }
    }
    None
}

fn unreferenced(written: &str) -> &str {
    let mut t = written.trim();
    while let Some(r) = t.strip_prefix('&') {
        t = r.trim_start();
        if t.starts_with('\'') {
            t = t.trim_start_matches(|c: char| c == '\'' || c == '_' || c.is_alphanumeric());
            t = t.trim_start();
        }
        t = t.strip_prefix("mut ").unwrap_or(t).trim_start();
    }
    t
}

fn type_args(written: &str) -> (String, Vec<String>) {
    let t = unreferenced(written);
    match t.find('<').and_then(|o| Some((t.get(..o)?, angled(t)?.0))) {
        Some((name, args)) => (
            name.trim()
                .rsplit("::")
                .next()
                .unwrap_or_default()
                .to_owned(),
            top_level(args).into_iter().map(str::to_owned).collect(),
        ),
        None => (
            t.rsplit("::").next().unwrap_or_default().to_owned(),
            Vec::new(),
        ),
    }
}

fn params(generics: &str) -> Vec<String> {
    top_level(generics)
        .into_iter()
        .filter(|p| !p.starts_with('\''))
        .filter_map(|p| {
            let p = p.strip_prefix("const ").unwrap_or(p);
            Some(p.split([':', '=']).next()?.trim().to_owned())
        })
        .collect()
}

fn substitute(
    ret: &str,
    impl_line: &str,
    fn_line: &str,
    word: &str,
    written: &str,
    krate: &str,
) -> String {
    let header = impl_line.trim_start();
    let header = header.strip_prefix("unsafe ").unwrap_or(header);
    let rest = header.strip_prefix("impl").unwrap_or_default();
    let (impl_params, self_ty) = match rest.trim_start().strip_prefix('<') {
        Some(_) => angled(rest).map_or((Vec::new(), ""), |(g, r)| (params(g), r)),
        None => (Vec::new(), rest),
    };
    let self_ty = self_ty.split(" for ").last().unwrap_or_default();
    let self_ty = self_ty.split(['{']).next().unwrap_or_default();
    let self_ty = self_ty.split(" where").next().unwrap_or_default();
    let (ty_name, ty_args) = type_args(self_ty);
    let (w_name, w_args) = type_args(written);
    let mut bound: HashMap<&str, &str> = HashMap::new();
    if ty_name == w_name {
        for (a, w) in ty_args.iter().zip(&w_args) {
            if impl_params.contains(a) && w != "_" {
                bound.insert(a, w);
            }
        }
    }
    let fn_params = fn_line
        .split_once(&format!("fn {word}"))
        .and_then(|(_, r)| r.trim_start().starts_with('<').then(|| angled(r)).flatten())
        .map(|(g, _)| params(g))
        .unwrap_or_default();
    let owned = unreferenced(written);
    let param = |t: &str| impl_params.iter().chain(&fn_params).any(|p| p == t);
    TOKEN
        .replace_all(ret, |c: &regex::Captures| {
            let t = &c[0];
            let first = t.split("::").next().unwrap_or_default();
            if t.starts_with('\'') {
                t.to_owned()
            } else if t.contains("::") && (first == "Self" || param(first)) {
                "_".to_owned()
            } else if t == "Self" {
                owned.to_owned()
            } else if let Some(b) = bound.get(t) {
                (*b).to_owned()
            } else if param(t) {
                "_".to_owned()
            } else if t.contains("::") || t.starts_with(|c: char| c.is_ascii_uppercase()) {
                format!("{krate}::{t}")
            } else {
                t.to_owned()
            }
        })
        .into_owned()
}

impl App {
    pub(super) fn rust_std_head(
        &self,
        file: &Path,
        text: &str,
        line0: usize,
        name: &str,
    ) -> Option<Got> {
        let lines: Vec<&str> = text.lines().collect();
        let bindings = search::bindings(Kind::Rust, text, line0 + 1, name);
        let (path, written, at) = match bindings.as_slice() {
            [b] => match search::rust_holds(&lines, b.line1 - 1, name)? {
                search::RustHolds::Type(w) => (file.to_path_buf(), w, b.line1 - 1),
                search::RustHolds::Call(parts) => {
                    let (decl, w) = self.rust_call_returns(file, text, b.line1 - 1, &parts)?;
                    (decl.path, w, decl.line1 - 1)
                }
                _ => return None,
            },
            [] => {
                let re = Regex::new(&format!(
                    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:const|static)\s+{}\s*:\s*([^=]+?)\s*=",
                    regex::escape(name)
                ))
                .ok()?;
                let mut found = lines
                    .iter()
                    .enumerate()
                    .filter_map(|(i, l)| re.captures(l).map(|c| (i, c[1].to_owned())));
                let (Some((i, w)), None) = (found.next(), found.next()) else {
                    return None;
                };
                (file.to_path_buf(), w, i)
            }
            _ => return None,
        };
        let bare = rust_bare_type(&written)?;
        let owner = self.text_of(&path)?;
        (DEREF.iter().any(|(t, _)| *t == bare) && self.rust_std_named(&owner, &bare)).then_some(
            Got::Written {
                file: path,
                written,
                line0: at,
            },
        )
    }

    pub(super) fn rust_std_returns(
        &self,
        file: &Path,
        written: &str,
        line0: usize,
        word: &str,
    ) -> Option<Got> {
        let bare = rust_bare_type(written)?;
        let decl = if DEREF.iter().any(|(t, _)| *t == bare) {
            let owner = self.text_of(file)?;
            let found = self.rust_std_found(&owner, "", &bare, word)?;
            let [Candidate { hit, .. }] = <[Candidate; 1]>::try_from(found).ok()?;
            hit
        } else {
            self.rust_std_marked(written, word)?
        };
        let krate = ["core", "alloc", "std"].into_iter().find(|k| {
            self.sysroot_src(k)
                .is_some_and(|s| decl.path.starts_with(&s.src))
        })?;
        let t = self.text_of(&decl.path)?;
        let lines: Vec<&str> = t.lines().collect();
        let m = search::rust_method_at(&lines, decl.line1)?;
        let ret = search::rust_return_type(&lines, decl.line1 - 1)?;
        let impl_line = lines.get(m.owner_line1 - 1)?;
        let sub = substitute(&ret, impl_line, &decl.text, word, written, krate);
        Some(self.rust_got(file, sub, line0))
    }

    fn rust_std_grep(
        &self,
        files: &[PathBuf],
        pattern: &str,
    ) -> Vec<(Hit, String, search::RustOwner)> {
        search::grep_project(&self.root, files, pattern, false, false, None, None)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|h| {
                let t = self.text_of(&h.path)?;
                if search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) == Some(&true) {
                    return None;
                }
                let lines: Vec<&str> = t.lines().collect();
                let (owner, line) = match search::rust_method_at(&lines, h.line1) {
                    Some(m) => (m.owner, lines.get(m.owner_line1 - 1)?.to_string()),
                    None => (search::RustOwner::Unreadable, String::new()),
                };
                Some((h, line, owner))
            })
            .collect()
    }

    pub(super) fn rust_std_marked(&self, written: &str, word: &str) -> Option<Hit> {
        let c = MARKED.captures(written.trim())?;
        let name = &c[2];
        let src = self.sysroot_src(&c[1])?;
        let decls = self.rust_std_grep(&src.files, &search::rust_type_decl_pattern(name));
        let [(decl, ..)] = <[_; 1]>::try_from(decls).ok()?;
        let method = search::rust_method_pattern(word);
        let own: Vec<Hit> = self
            .rust_std_grep(std::slice::from_ref(&decl.path), &method)
            .into_iter()
            .filter(|(_, line, owner)| {
                !matches!(
                    owner,
                    search::RustOwner::Trait(_) | search::RustOwner::Unreadable
                ) && search::rust_impl_type(line).as_deref() == Some(name)
            })
            .map(|(h, ..)| h)
            .collect();
        let found = if own.is_empty() {
            let t = self.text_of(&decl.path)?;
            let traits: Vec<String> = t
                .lines()
                .filter(|l| search::rust_impl_type(l).as_deref() == Some(name))
                .filter_map(search::rust_impl_trait)
                .collect();
            let mut files = Vec::new();
            for krate in ["core", "alloc", "std"] {
                files.extend(self.sysroot_src(krate)?.files);
            }
            self.rust_std_grep(&files, &method)
                .into_iter()
                .filter(|(_, _, owner)| {
                    matches!(owner, search::RustOwner::Trait(t) if traits.contains(t))
                })
                .map(|(h, ..)| h)
                .collect()
        } else {
            own
        };
        let [hit] = <[Hit; 1]>::try_from(found).ok()?;
        Some(hit)
    }
}
