use super::{App, Kind, Path, Regex, Typed, bound, hidden, names_itself, search};

impl App {
    pub(super) fn written_type(
        &self,
        kind: Kind,
        file: &Path,
        text: &str,
        b: &search::Binding,
        t: &str,
    ) -> Option<Typed> {
        let path = search::type_path(kind, t).filter(|p| p.len() > 1);
        let imported = imported_name(kind, text, t);
        if path.is_some_and(|p| hidden(kind, file, text, b.line1, &p[0]))
            || imported.is_some_and(|n| shadows_import(kind, text, b.line1, n))
        {
            return None;
        }
        let constructed = matches!(b.value, search::Value::New(_));
        if let Some(ty) =
            imported.and_then(|n| local_class(kind, file, text, b.line1, n, constructed))
        {
            return Some(ty);
        }
        match &b.value {
            search::Value::Type(_) => self.annotated_type(kind, file, text, b.line1, t),
            _ => self.type_decl(kind, file, t),
        }
    }
}

fn imported_name<'a>(kind: Kind, text: &str, written: &'a str) -> Option<&'a str> {
    let simple = kind == Kind::TsJs && search::type_path(kind, written)?.len() == 1;
    (simple && bound(&search::imports(kind, text), written).is_some()).then_some(written)
}

fn local_class(
    kind: Kind,
    file: &Path,
    text: &str,
    line1: usize,
    name: &str,
    constructed: bool,
) -> Option<Typed> {
    let keyword = if constructed {
        "(?:class|function)"
    } else {
        "class"
    };
    let class = Regex::new(&format!(
        r"^\s*(?:export\s+)?(?:abstract\s+)?{keyword}\s+{}\b",
        regex::escape(name)
    ))
    .expect("an escaped name keeps the pattern valid");
    let lines: Vec<&str> = text.lines().collect();
    search::bindings(kind, text, line1, name)
        .iter()
        .find(|b| lines.get(b.line1 - 1).is_some_and(|l| class.is_match(l)))
        .map(|b| Typed {
            name: name.to_owned(),
            path: file.to_path_buf(),
            line: b.line1,
        })
}

fn shadows_import(kind: Kind, text: &str, line1: usize, name: &str) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    search::bindings(kind, text, line1, name).iter().any(|b| {
        let statement = lines.get(b.line1 - 1..).unwrap_or_default();
        let end = statement
            .iter()
            .position(|l| l.contains(';') || l.contains(')'))
            .map_or(1, |i| i + 1);
        let first = statement.first().unwrap_or(&"");
        (search::js_constructor(first) == Some(name) && prototyped_elsewhere(&lines, b.line1, name))
            || !names_itself(kind, first, name)
                && !statement
                    .iter()
                    .take(end.min(8))
                    .any(|l| l.contains("require(") || l.contains("import("))
    })
}

fn prototyped_elsewhere(lines: &[&str], decl: usize, name: &str) -> bool {
    let prototype = Regex::new(&format!(r"^\s*{}\.prototype\b", regex::escape(name)))
        .expect("an escaped name keeps the pattern valid");
    let indent = |l: &str| l.len() - l.trim_start().len();
    let depth = lines.get(decl - 1).map_or(0, |l| indent(l));
    let outer = |l: &&str| !l.trim().is_empty() && indent(l) < depth;
    let start = lines
        .get(..decl.saturating_sub(1))
        .and_then(|above| above.iter().rposition(outer))
        .map_or(0, |i| i + 1);
    let end = lines
        .get(decl..)
        .and_then(|rest| rest.iter().position(outer))
        .map_or(lines.len(), |i| decl + i);
    (1..)
        .zip(lines)
        .any(|(n, l)| (n <= start || n > end) && prototype.is_match(l))
}
