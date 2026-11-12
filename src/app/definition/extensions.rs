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
        if hit.path == here {
            return Some(true);
        }
        let kotlin_or_scala = here
            .extension()
            .is_some_and(|e| e == "kt" || e == "kts" || e == "scala");
        match kind {
            Kind::Jvm if kotlin_or_scala => {
                self.jvm_receiver_of(hit, word)?;
                Some(self.jvm_package_sees(here, text, word, &hit.path))
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
                let name = search::rust_impl_trait(enclosing(&there, hit.line1)?)?;
                rust_sees_trait(text, &hit.path, &name)
            }
            _ => None,
        }
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

fn rust_sees_trait(text: &str, there: &Path, name: &str) -> Option<bool> {
    let stem = there.file_stem()?.to_str()?;
    let module = if stem == "mod" {
        there.parent()?.file_name()?.to_str()?
    } else {
        stem
    };
    let n = regex::escape(name);
    let named = Regex::new(&format!(r"(\w+)::(?:{n}\b|\{{[^}}]*\b{n}\b)")).ok()?;
    let declared = Regex::new(&format!(r"^\s*(?:pub\S*\s+)?trait\s+{n}\b")).ok()?;
    let used = Regex::new(&format!(
        r"\b{}::(?:{n}\b|\{{[^}}]*\b{n}\b)",
        regex::escape(module)
    ))
    .ok()?;
    let uses: Vec<&str> = text
        .lines()
        .filter(|l| l.trim_start().starts_with("use ") || l.trim_start().starts_with("pub use "))
        .collect();
    if text.lines().any(|l| declared.is_match(l)) {
        return None;
    }
    if uses.iter().any(|l| used.is_match(l)) {
        return Some(true);
    }
    let unsure = uses.iter().any(|l| {
        l.contains('*')
            || (named.captures_iter(l)).any(|c| matches!(&c[1], "crate" | "self" | "super"))
    });
    (!unsure).then_some(false)
}

fn indent(l: &str) -> usize {
    l.len() - l.trim_start().len()
}
