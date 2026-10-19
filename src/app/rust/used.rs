use std::sync::LazyLock;

use super::{App, Hit, InCrate, Kind, MacroUse, Path, Regex, Typed, declares_mod, search};

impl App {
    pub(super) fn rust_used_type(
        &self,
        file: &Path,
        text: &str,
        name: &str,
        inline_mod: Option<usize>,
    ) -> Option<Typed> {
        static DECL: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|union)\s+([A-Za-z_]\w*)")
                .expect("the regex is a literal")
        });
        let lines: Vec<&str> = text.lines().collect();
        let literal = search::literal_lines(Kind::Rust, text);
        let in_scope = |u: &search::RustUse| {
            inline_mod.is_none_or(|m| {
                let line0 = text
                    .get(..u.start_byte)
                    .map_or(0, |t| t.matches('\n').count());
                search::rust_inline_mod_past(&lines, &literal, line0) == Some(m)
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
            Some((root, rest)) if root == "crate" || root == "super" => (root.as_str(), rest),
            Some((root, _)) if declares_mod(text, root) => ("self", used.path.as_slice()),
            Some((root, rest)) if root != "self" => (root.as_str(), rest),
            _ => return None,
        };
        let (krate, module) = match root {
            "crate" | "super" | "self" => self.rust_own_crate(file, root, rest)?,
            _ => (self.workspace_crate(root)?, Vec::new()),
        };
        let rest: Vec<String> = rest.iter().filter(|p| *p != "super").cloned().collect();
        let InCrate {
            hits, proven: true, ..
        } = self.rust_in_crate(&krate, &module, &rest, MacroUse::NotMacro, 0)?
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
}
