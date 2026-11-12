use super::{App, Candidate, Hit, Kind, Path, Reason, Regex, search};

impl App {
    pub(super) fn imported_extensions(
        &self,
        kind: Kind,
        word: &str,
        here: &Path,
        found: &mut Vec<Candidate>,
    ) {
        if found.len() < 2
            || !found
                .iter()
                .all(|c| matches!(c.reason, Reason::Receiver(_)))
        {
            return;
        }
        let text = self.buf.lines.join("\n");
        let Some(seen) = (found.iter())
            .map(|c| self.sees_extension(kind, here, &text, word, &c.hit))
            .collect::<Option<Vec<bool>>>()
        else {
            return;
        };
        if seen.contains(&true) && seen.contains(&false) {
            let mut seen = seen.into_iter();
            found.retain(|_| seen.next().unwrap_or(true));
        }
    }

    fn sees_extension(
        &self,
        kind: Kind,
        here: &Path,
        text: &str,
        word: &str,
        hit: &Hit,
    ) -> Option<bool> {
        if hit.path == here && matches!(kind, Kind::Swift | Kind::CSharp) {
            return Some(true);
        }
        let kotlin_or_scala = here
            .extension()
            .is_some_and(|e| e == "kt" || e == "kts" || e == "scala");
        match kind {
            Kind::Jvm if kotlin_or_scala => {
                self.jvm_receiver_of(hit, word)?;
                self.jvm_sees_extension(here, text, word, hit)
            }
            Kind::Swift => {
                let there = self.text_of(&hit.path)?;
                search::swift_extension(enclosing(&there, hit.line1)?).then_some(())?;
                let mut kept = vec![hit.clone()];
                self.swift_imported_modules(here, text, &mut kept);
                Some(!kept.is_empty())
            }
            Kind::CSharp => {
                let this = Regex::new(r"\(\s*this\s").expect("the regex is a literal");
                this.is_match(&hit.text).then_some(())?;
                self.cs_sees_namespace_of(here, word, hit)
            }
            Kind::Rust => {
                let there = self.text_of(&hit.path)?;
                rust_sees_trait(text, &hit.path, &there, enclosing(&there, hit.line1)?)
            }
            _ => None,
        }
    }

    fn jvm_sees_extension(&self, here: &Path, text: &str, word: &str, hit: &Hit) -> Option<bool> {
        let there = self.text_of(&hit.path)?;
        let line = there.lines().nth(hit.line1.checked_sub(1)?)?;
        let top = indent(line) == 0
            || enclosing(&there, hit.line1)
                .is_some_and(|l| indent(l) == 0 && l.starts_with("extension"));
        let packages = text
            .lines()
            .filter(|l| l.trim_start().starts_with("package "))
            .count();
        if !top || packages > 1 {
            return None;
        }
        let declared = search::jvm_package(&there).unwrap_or_default();
        if hit.path == here || declared == search::jvm_package(text).unwrap_or_default() {
            return Some(true);
        }
        let imports = search::jvm_imports(here, text);
        let names = |i: &search::JvmImport| {
            i.path.join(".") == format!("{declared}.{}", i.bound_name)
                && [word, "*", "_"].contains(&i.bound_name.as_str())
        };
        if imports.iter().any(names) {
            return Some(true);
        }
        let relative = |i: &search::JvmImport| {
            let prefix = i.path[..i.path.len().saturating_sub(1)].join(".");
            !prefix.is_empty() && declared.ends_with(&format!(".{prefix}"))
        };
        let selectors = text
            .lines()
            .any(|l| l.trim_start().starts_with("import ") && l.contains('{'));
        (!selectors && !imports.iter().any(relative)).then_some(false)
    }
}

fn enclosing(text: &str, line1: usize) -> Option<&str> {
    let lines: Vec<&str> = text.lines().collect();
    let depth = indent(lines.get(line1.checked_sub(1)?)?);
    lines
        .get(..line1 - 1)?
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty() && indent(l) < depth)
        .copied()
}

fn rust_sees_trait(text: &str, there: &Path, there_text: &str, imp: &str) -> Option<bool> {
    static USE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s[^;]*;").expect("the regex is a literal")
    });
    let name = search::rust_impl_trait(imp)?;
    let n = regex::escape(&name);
    let declared = Regex::new(&format!(r"(?m)^\s*(?:pub\S*\s+)?trait\s+{n}\b")).ok()?;
    let named = Regex::new(&format!(r"(\w+)::(?:{n}\b|\{{[^}}]*\b{n}\b)")).ok()?;
    if declared.is_match(text) {
        return None;
    }
    let stem = there.file_stem()?.to_str()?;
    let own = if stem == "mod" {
        there.parent()?.file_name()?.to_str()?
    } else {
        stem
    };
    let module = match named.captures(imp) {
        Some(c) => c[1].to_owned(),
        None if declared.is_match(there_text) => own.to_owned(),
        None => (USE.find_iter(there_text))
            .find_map(|u| named.captures(u.as_str()))
            .map(|c| c[1].to_owned())?,
    };
    if matches!(module.as_str(), "crate" | "self" | "super") {
        return None;
    }
    let uses: Vec<&str> = USE.find_iter(text).map(|m| m.as_str()).collect();
    if uses
        .iter()
        .flat_map(|u| named.captures_iter(u))
        .any(|c| c[1] == module)
    {
        return Some(true);
    }
    let unsure = uses.iter().any(|u| {
        u.contains('*')
            || (named.captures_iter(u)).any(|c| matches!(&c[1], "crate" | "self" | "super"))
    });
    (!unsure).then_some(false)
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}
