use std::sync::LazyLock;

use super::chain::{let_initializer, opening, trailing_name};
use super::returns;
use super::{
    App, Candidate, HashMap, Hit, InCrate, Kind, MacroUse, Path, Reason, Regex, Typed, Typing,
    bound, declares_mod, rust_bare_type, rust_written_type, search,
};

static SLICE_IMPL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:(?:unsafe|const)\s+)*impl\s*<[^{]*?>\s+(?:[\w:]+(?:<[^{]*?>)?\s+for\s+)?(\[T(?:;\s*N)?\])\s*(?:\{|where\b|$)")
        .expect("the regex is a literal")
});

pub(super) const DEREF: &[(&str, &[&str])] = &[
    ("String", &["String", "str"]),
    ("str", &["str"]),
    ("PathBuf", &["PathBuf", "Path"]),
    ("Path", &["Path"]),
    ("OsString", &["OsString", "OsStr"]),
    ("OsStr", &["OsStr"]),
    ("Vec", &["Vec", "[T]"]),
    ("VecDeque", &["VecDeque"]),
    ("Option", &["Option"]),
    ("Result", &["Result"]),
    ("HashMap", &["HashMap"]),
    ("HashSet", &["HashSet"]),
    ("BTreeMap", &["BTreeMap"]),
    ("BTreeSet", &["BTreeSet"]),
    ("[T; N]", &["[T; N]", "[T]"]),
    ("[T]", &["[T]"]),
];

static INDEXED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:&\s*(?:'\w+\s+)?(?:mut\s+)?)?(?:(?:(?:std|alloc)::(?:vec|collections)::)?(Vec|VecDeque)\s*<(.+)>|\[(.+)\])$")
        .expect("the regex is a literal")
});

static STD_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:&\s*(?:'\w+\s+)?(?:mut\s+)?)*((?:\w+::)*)([A-Z]\w*)")
        .expect("the regex is a literal")
});

static VARIANT_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^((?:\w+::)*[A-Z]\w*)::([A-Z]\w*)\s*(?:\{([^{}]*)\}|\(([^()]*)\))\s*")
        .expect("the regex is a literal")
});

static PATTERN_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:=>|=[^=]|if\s)").expect("the regex is a literal"));

#[derive(PartialEq, Eq, Debug)]
enum Field {
    Named(String),
    At(usize, usize),
}

fn rust_variant_bindings(line: &str, name: &str) -> Option<Vec<(String, String, Field)>> {
    let bare = |s: &str| {
        let s = s.trim();
        let s = s.strip_prefix("ref ").unwrap_or(s).trim_start();
        s.strip_prefix("mut ").unwrap_or(s).trim().to_owned()
    };
    let t = line.trim_start();
    let mut t = ["} else if let ", "if let ", "while let ", "let "]
        .iter()
        .find_map(|p| t.strip_prefix(p))
        .unwrap_or(t);
    let mut found = Vec::new();
    loop {
        let c = VARIANT_PATTERN.captures(t)?;
        let field = if let Some(named) = c.get(3) {
            Field::Named(named.as_str().split(',').find_map(|p| {
                let (field, bound) = p.split_once(':').unwrap_or((p, p));
                (bare(bound) == name).then(|| bare(field))
            })?)
        } else {
            let parts: Vec<String> = c.get(4)?.as_str().split(',').map(bare).collect();
            if !parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|ch| ch.is_alphanumeric() || ch == '_'))
            {
                return None;
            }
            Field::At(parts.iter().position(|p| p == name)?, parts.len())
        };
        found.push((c[1].to_owned(), c[2].to_owned(), field));
        t = t.get(c.get(0)?.end()..)?;
        match t.strip_prefix('|') {
            Some(next) if !next.starts_with('|') => t = next.trim_start(),
            _ => break,
        }
    }
    let rest = t.get(PATTERN_END.find(t)?.end()..)?;
    if rest.contains('|') || ["let ", "for ", "=>"].iter().any(|k| rest.contains(k)) {
        return None;
    }
    Some(found)
}

fn pattern_lines(lines: &[&str], at: usize) -> String {
    let mut start = at;
    while let Some(above) = start.checked_sub(1).map(|i| lines[i].trim_end()) {
        let led = lines[start].trim_start().starts_with('|')
            && !(above.ends_with(['{', ',']) || above.trim_start() == "}" || above.contains("=>"));
        if !(led || above.ends_with('|') && !above.ends_with("||")) {
            break;
        }
        start -= 1;
    }
    let joined = lines[start..=at]
        .iter()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join(" ");
    match joined.strip_prefix('|') {
        Some(rest) if start < at => rest.trim_start().to_owned(),
        _ => joined,
    }
}

pub(super) fn tuple_element(decl: &str, name: &str, index: &str) -> Option<String> {
    static ELEMENT: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^(?:pub(?:\([^)]*\))?\s+)?([^#]+)$").expect("the regex is a literal")
    });
    let tuple = Regex::new(&format!(
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?struct\s+{}\s*(?:<.*>)?\s*\((.*)\)\s*;\s*$",
        regex::escape(name)
    ))
    .ok()?;
    let inner = tuple.captures(decl)?.get(1)?.as_str();
    let part = *returns::top_level(inner).get(index.parse::<usize>().ok()?)?;
    Some(ELEMENT.captures(part)?[1].trim().to_owned())
}

pub(super) fn rust_element(written: &str) -> Option<(Option<String>, String)> {
    let c = INDEXED.captures(written.trim())?;
    let (container, args, stop) = match c.get(1) {
        Some(m) => (Some(m.as_str().to_owned()), c.get(2)?.as_str(), b','),
        None => (None, c.get(3)?.as_str(), b';'),
    };
    let mut depth = 0i32;
    let end = args
        .bytes()
        .position(|b| {
            match b {
                b'<' | b'(' | b'[' => depth += 1,
                b'>' | b')' | b']' => depth -= 1,
                _ => {}
            }
            depth == 0 && b == stop
        })
        .unwrap_or(args.len());
    Some((container, args.get(..end)?.trim().to_owned()))
}

impl App {
    pub(super) fn rust_std_written(
        &self,
        file: &Path,
        text: &str,
        line0: usize,
        name: &str,
    ) -> Option<String> {
        let lines: Vec<&str> = text.lines().collect();
        if name == "self" {
            return rust_bare_type(&search::rust_self_type(&lines, line0)?.written);
        }
        if let Some(written) = rust_written_type(text, line0, name) {
            return Some(written);
        }
        let bindings = search::bindings(Kind::Rust, text, line0 + 1, name);
        let [b] = bindings.as_slice() else {
            return None;
        };
        if let Some(item) = self.rust_for_item(file, text, b.line1 - 1, name) {
            return Some(item);
        }
        if let Some(alternatives) = rust_variant_bindings(&pattern_lines(&lines, b.line1 - 1), name)
        {
            let mut types = alternatives
                .iter()
                .map(|(o, v, f)| self.rust_variant_field(file, o, v, f, b.line1 - 1));
            let first = types.next()??;
            return types.all(|t| t.as_ref() == Some(&first)).then_some(first);
        }
        let held = search::rust_tuple_element(&lines, b.line1 - 1, name)
            .or_else(|| let_initializer(&lines, b.line1 - 1, name));
        if let Some(held) = held
            && let Some(inner) = held.strip_prefix("vec!")
            && inner.starts_with('[')
            && inner.ends_with(']')
            && opening(inner, b']', b'[') == Some(0)
            && self
                .grep(r"macro_rules!\s*vec\b", false, false, |p| {
                    p.extension().is_some_and(|e| e == "rs")
                })
                .is_ok_and(|h| h.is_empty())
        {
            return Some("Vec".to_owned());
        }
        match search::rust_holds(&lines, b.line1 - 1, name) {
            Some(search::RustHolds::AssocCall {
                written_type,
                function,
            }) if matches!(
                function.as_str(),
                "new" | "from" | "with_capacity" | "default"
            ) =>
            {
                rust_bare_type(&written_type)
            }
            Some(_) => None,
            None => self.rust_let_written(file, text, b.line1 - 1, name),
        }
    }

    pub(super) fn rust_variant_typed(
        &self,
        file: &Path,
        lines: &[&str],
        at: usize,
        name: &str,
    ) -> Option<Typed> {
        let alternatives = rust_variant_bindings(&pattern_lines(lines, at), name)?;
        let mut types = alternatives.iter().map(|(o, v, f)| {
            let (owner, written) = self.rust_variant_written(file, o, v, f, at)?;
            self.rust_resolve(&owner.path, &written, owner.line - 1)
        });
        let first = types.next()??;
        types
            .all(|t| t.is_some_and(|t| (&t.path, t.line) == (&first.path, first.line)))
            .then_some(first)
    }

    fn rust_variant_field(
        &self,
        file: &Path,
        owner: &str,
        variant: &str,
        field: &Field,
        at: usize,
    ) -> Option<String> {
        let (ty, written) = self.rust_variant_written(file, owner, variant, field, at)?;
        let text = self.text_of(&ty.path)?;
        let imports = search::imports(Kind::Rust, &text);
        rust_bare_type(&written).filter(|b| {
            STD_PATH.captures(written.trim()).is_none_or(|c| {
                let first = c[1].split("::").next().unwrap_or_default();
                let root = bound(&imports, first)
                    .and_then(|p| p.into_iter().next())
                    .unwrap_or_else(|| first.to_owned());
                matches!(root.as_str(), "" | "std" | "core" | "alloc")
            }) && self.rust_std_named(&text, b)
        })
    }

    fn rust_variant_written(
        &self,
        file: &Path,
        owner: &str,
        variant: &str,
        field: &Field,
        at: usize,
    ) -> Option<(Typed, String)> {
        let ty = self.rust_resolve(file, owner, at)?;
        let text = self.text_of(&ty.path)?;
        let lines: Vec<&str> = text.lines().collect();
        let indent = |l: &str| l.len() - l.trim_start().len();
        let decl = lines.get(ty.line - 1)?;
        if !decl.contains("enum ") {
            return None;
        }
        let body = &lines[ty.line..];
        let end = body
            .iter()
            .position(|l| !l.trim().is_empty() && indent(l) <= indent(decl))?;
        let field = match field {
            Field::Named(field) => field,
            Field::At(i, n) => {
                let tuple =
                    Regex::new(&format!(r"^\s+{}\s*\((.*)\),?\s*$", regex::escape(variant)))
                        .ok()?;
                let written = body[..end].iter().find_map(|l| tuple.captures(l))?;
                let types = returns::top_level(written.get(1)?.as_str());
                if types.len() != *n {
                    return None;
                }
                let written = types.get(*i)?.trim().to_owned();
                return Some((ty, written));
            }
        };
        let one_line = Regex::new(&format!(
            r"^\s+{}\s*\{{([^{{}}]*)\}},?\s*$",
            regex::escape(variant)
        ))
        .ok()?;
        if let Some(c) = body[..end].iter().find_map(|l| one_line.captures(l)) {
            let mut written = returns::top_level(c.get(1)?.as_str())
                .into_iter()
                .filter_map(|p| p.split_once(':'))
                .filter(|(f, _)| f.trim() == field.as_str());
            let (Some((_, w)), None) = (written.next(), written.next()) else {
                return None;
            };
            let written = w.trim().to_owned();
            return Some((ty, written));
        }
        let head = Regex::new(&format!(r"^\s+{}\s*\{{\s*$", regex::escape(variant))).ok()?;
        let named = Regex::new(&format!(r"^\s+{}\s*:\s*(.+?),?\s*$", regex::escape(field))).ok()?;
        let start = body[..end].iter().position(|l| head.is_match(l))?;
        let close = start
            + body[start..end]
                .iter()
                .position(|l| l.trim().starts_with('}'))?;
        let written = body[start + 1..close]
            .iter()
            .find_map(|l| named.captures(l).map(|c| c[1].to_owned()))?;
        Some((ty, written))
    }

    pub(super) fn rust_std_typed(&self, file: &Path, written: &str) -> Option<(String, String)> {
        let text = self.text_of(file)?;
        let Some(c) = STD_PATH.captures(written.trim()) else {
            return Some((text, rust_bare_type(written)?));
        };
        let mut path: Vec<String> = c[1]
            .split("::")
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        if let Some(full) = path
            .first()
            .and_then(|f| bound(&search::imports(Kind::Rust, &text), f))
        {
            path.splice(..1, full);
        }
        match path.first().map(String::as_str) {
            None => return Some((text, c[2].to_owned())),
            Some("std" | "core" | "alloc") => return Some((String::new(), c[2].to_owned())),
            Some(_) => {}
        }
        path.push(c[2].to_owned());
        let (root, rest) = match path[0].as_str() {
            "crate" | "self" | "super" => (path[0].as_str(), &path[1..]),
            first if declares_mod(&text, first) => ("self", &path[..]),
            _ => return None,
        };
        let (krate, module) = self.rust_own_crate(file, root, rest)?;
        let rest: Vec<String> = rest.iter().filter(|p| *p != "super").cloned().collect();
        let InCrate {
            hits, proven: true, ..
        } = self.rust_in_crate(&krate, &module, &rest, MacroUse::NotMacro, 0)?
        else {
            return None;
        };
        let [alias] = <[Hit; 1]>::try_from(hits).ok()?;
        let named = Regex::new(&format!(
            r"^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+{}\b",
            regex::escape(&c[2])
        ))
        .ok()?;
        if !named.is_match(&alias.text) {
            return None;
        }
        let owner = self.text_of(&alias.path)?;
        let statement = owner
            .lines()
            .skip(alias.line1 - 1)
            .take(4)
            .collect::<Vec<_>>()
            .join(" ");
        let alias = statement.split_once(';')?.0;
        let mut depth = 0_i32;
        let eq = alias.find(|c| {
            depth += i32::from(c == '<') - i32::from(c == '>');
            c == '=' && depth == 0
        })?;
        let target = alias.get(eq + 1..)?;
        let t = STD_PATH.captures(target.trim())?;
        let std = matches!(t[1].split("::").next(), Some("" | "std" | "core" | "alloc"));
        std.then(|| (owner, t[2].to_owned()))
    }

    pub(in crate::app) fn rust_array_call(
        &mut self,
        text: &str,
        before: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let receiver = before.strip_suffix('.')?.trim_end();
        if !receiver.ends_with(']') {
            return None;
        }
        let open = opening(receiver, b']', b'[')?;
        let lead = receiver.get(..open)?.trim_end();
        if !(lead.is_empty() || lead.ends_with(['(', ',', '=', '{', ';', '&', '[', '|'])) {
            return None;
        }
        self.rust_std_call(text, "[..]", "[T; N]", word)
    }

    pub(super) fn rust_std_call(
        &mut self,
        text: &str,
        name: &str,
        std: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        self.external_files(Kind::Rust);
        self.rust_std_found(text, name, std, word)
    }

    pub(super) fn rust_std_found(
        &self,
        text: &str,
        name: &str,
        std: &str,
        word: &str,
    ) -> Option<Vec<Candidate>> {
        let (_, levels) = DEREF.iter().find(|(t, _)| *t == std)?;
        if !self.rust_std_named(text, std) {
            return None;
        }
        let mut files = Vec::new();
        for krate in ["core", "alloc", "std"] {
            let Some(krate) = self.sysroot_src(krate) else {
                return Some(Vec::new());
            };
            files.extend(krate.files);
        }
        let hits = search::grep_project(
            &self.root,
            &files,
            &search::rust_method_pattern(word),
            false,
            false,
            None,
            None,
        )
        .unwrap_or_default();
        let mut owned: Vec<(Hit, bool, String)> = Vec::new();
        for h in hits {
            let Some(t) = self.text_of(&h.path) else {
                continue;
            };
            let lines: Vec<&str> = t.lines().collect();
            if search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) == Some(&true) {
                continue;
            }
            let Some(m) = search::rust_method_at(&lines, h.line1) else {
                continue;
            };
            let owner = lines[m.owner_line1 - 1];
            let slice = SLICE_IMPL
                .captures(owner)
                .map(|c| if c[1].contains(';') { "[T; N]" } else { "[T]" }.to_owned());
            let stable = search::rust_stability(&lines, h.line1);
            let inherent = match m.owner {
                search::RustOwner::Inherent if stable => true,
                search::RustOwner::Unreadable if stable && slice.is_some() => true,
                search::RustOwner::ImplOf(_) => false,
                _ => continue,
            };
            if let Some(ty) = slice.or_else(|| search::rust_impl_type(owner)) {
                owned.push((h, inherent, ty));
            }
        }
        let found: Vec<Hit> = levels
            .iter()
            .find_map(|level| {
                let at: Vec<&(Hit, bool, String)> =
                    owned.iter().filter(|(_, _, t)| t == level).collect();
                let inherent: Vec<Hit> = at.iter().filter(|o| o.1).map(|o| o.0.clone()).collect();
                let found = if inherent.is_empty() && (std, *level) != ("Vec", "[T]") {
                    at.into_iter().map(|o| o.0.clone()).collect()
                } else {
                    inherent
                };
                (!found.is_empty()).then_some(found)
            })
            .unwrap_or_default();
        let reason = Reason::Receiver(format!("{name}: {std}"));
        Some(
            found
                .into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
        )
    }

    pub(super) fn rust_std_named(&self, text: &str, std: &str) -> bool {
        if bound(&search::imports(Kind::Rust, text), std).is_some_and(|p| {
            p.first()
                .is_none_or(|f| !matches!(f.as_str(), "std" | "core" | "alloc"))
        }) {
            return false;
        }
        let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
        self.grep(&search::rust_type_decl_pattern(std), false, false, rs)
            .is_ok_and(|own| own.is_empty())
    }

    pub(super) fn rust_std_hides(
        &self,
        owner: &search::RustOwner,
        std: &str,
        seen: &mut HashMap<String, bool>,
    ) -> bool {
        match owner {
            search::RustOwner::Inherent => true,
            search::RustOwner::Trait(t) | search::RustOwner::ImplOf(t) => !*seen
                .entry(t.clone())
                .or_insert_with(|| self.rust_std_impl(t, std)),
            search::RustOwner::Unreadable => false,
        }
    }

    fn rust_std_impl(&self, tr: &str, std: &str) -> bool {
        let levels = DEREF
            .iter()
            .find(|(t, _)| *t == std)
            .map_or(&[][..], |(_, l)| *l);
        let Ok(re) = Regex::new(&format!(
            r"^\s*(?:unsafe\s+)?impl\s*(?:<(.*?)>)?\s+(?:\w+::)*{}\b(?:<.*?>)?\s+for\s+([^{{]+?)\s*(?:\bwhere\b.*|\{{.*)?$",
            regex::escape(tr)
        )) else {
            return true;
        };
        let rs = |p: &Path| p.extension().is_some_and(|e| e == "rs");
        let alias = |b: &str| {
            let found = |p: &str| {
                self.grep(p, false, false, rs)
                    .map_or(true, |h| !h.is_empty())
            };
            let pattern = format!(
                r"^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+{}\b",
                regex::escape(b)
            );
            found(&pattern) && !found(&search::rust_type_decl_pattern(b))
        };
        let headers = format!(
            r"^\s*(?:unsafe\s+)?impl\b(?:\s*<.*>)?\s*(?:\w+::)*{0}\b|^\s*>\s*(?:\w+::)*{0}\b",
            regex::escape(tr)
        );
        self.grep(&headers, false, false, rs).map_or(true, |hits| {
            hits.iter().any(|h| {
                let Some(c) = re.captures(&h.text) else {
                    return true;
                };
                let ty = &c[2];
                if ty.contains('$') {
                    return true;
                }
                let generic = |b: &str| {
                    c.get(1).is_some_and(|g| {
                        g.as_str()
                            .split(',')
                            .any(|p| p.split(':').next().map(str::trim) == Some(b))
                    })
                };
                match rust_bare_type(ty) {
                    Some(b) => levels.contains(&b.as_str()) || generic(&b) || alias(&b),
                    None => levels.contains(&if ty.contains(';') { "[T; N]" } else { "[T]" }),
                }
            })
        })
    }

    pub(in crate::app) fn rust_index_member(
        &mut self,
        here: &Path,
        text: &str,
        before: &str,
        word: &str,
        call: bool,
    ) -> Option<Typing> {
        let receiver = before.strip_suffix('.')?.trim_end();
        let inner = receiver.strip_suffix(']')?;
        let open = opening(receiver, b']', b'[')?;
        let index = inner.get(open + 1..)?.trim();
        let head = receiver.get(..open)?;
        let name = trailing_name(head);
        let lead = head.get(..head.len() - name.len())?.trim_end();
        if name.is_empty()
            || name.starts_with(|c: char| c.is_ascii_digit())
            || lead.ends_with(['.', ':', '!', '#'])
            || index.is_empty()
            || index.contains("..")
            || (call && !index.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
        let lines: Vec<&str> = text.lines().collect();
        let bindings = search::bindings(Kind::Rust, text, self.line + 1, name);
        let [b] = bindings.as_slice() else {
            return None;
        };
        let at = b.line1 - 1;
        let (file, written, line0) = match search::rust_holds(&lines, at, name)? {
            search::RustHolds::Type(w) => (here.to_path_buf(), w, at),
            search::RustHolds::Call(parts) => {
                let (decl, returns) = self.rust_call_returns(here, text, at, &parts)?;
                (decl.path, returns, decl.line1 - 1)
            }
            _ => return None,
        };
        let owner = self.text_of(&file)?;
        let (container, element) = rust_element(&written)?;
        if container.is_some_and(|c| !self.rust_std_named(&owner, &c)) {
            return None;
        }
        let via = format!("{name}[{index}]");
        let hits = match self.rust_resolve(&file, &element, line0) {
            Some(ty) if call => self.rust_members(&ty, word),
            Some(ty) => self
                .rust_field(&ty, word)
                .map(|(hit, _)| vec![hit])
                .unwrap_or_default(),
            None if call => {
                let std = rust_bare_type(&element)?;
                return self
                    .rust_std_call(&owner, &via, &std, word)
                    .map(|found| Typing::Std(found, std));
            }
            None => vec![self.rust_std_field(&owner, &element, word)?],
        };
        let reason = Reason::Receiver(format!("{via}: {element}"));
        Some(Typing::Members(
            hits.into_iter()
                .map(|hit| Candidate {
                    hit,
                    reason: reason.clone(),
                })
                .collect(),
        ))
    }

    fn rust_std_field(&mut self, text: &str, written: &str, word: &str) -> Option<Hit> {
        let c = STD_PATH.captures(written.trim())?;
        let mut path: Vec<String> = c[1]
            .split("::")
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        path.push(c[2].to_owned());
        if let Some(full) = bound(&search::imports(Kind::Rust, text), &path[0]) {
            path.splice(..1, full);
        }
        let (name, modules) = path.split_last()?;
        let (krate, modules) = modules.split_first()?;
        if !matches!(krate.as_str(), "std" | "core" | "alloc") || modules.is_empty() {
            return None;
        }
        let dir = modules.join("/");
        let file = format!("{dir}.rs");
        let mut files = Vec::new();
        for src in ["core", "alloc", "std"]
            .map(|k| self.sysroot_crate(k))
            .into_iter()
            .flatten()
        {
            files.extend(src.files.into_iter().filter(|f| {
                f.strip_prefix(&src.src)
                    .is_ok_and(|r| r.starts_with(&dir) || r == Path::new(&file))
            }));
        }
        let decls: Vec<Hit> = search::grep_project(
            &self.root,
            &files,
            &search::rust_type_decl_pattern(name),
            false,
            false,
            None,
            None,
        )
        .unwrap_or_default()
        .into_iter()
        .filter(|h| {
            self.text_of(&h.path).is_some_and(|t| {
                search::literal_lines(Kind::Rust, &t).get(h.line1 - 1) != Some(&true)
            })
        })
        .collect();
        let [decl] = <[Hit; 1]>::try_from(decls).ok()?;
        let t = self.text_of(&decl.path)?;
        let lines: Vec<&str> = t.lines().collect();
        let field = search::rust_struct_field(&lines, decl.line1 - 1, word)?;
        Some(Hit {
            path: decl.path,
            line1: field.line0 + 1,
            byte_col: None,
            text: lines[field.line0].to_owned(),
            deleted: None,
        })
    }
}
