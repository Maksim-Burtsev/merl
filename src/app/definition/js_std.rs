use super::{App, Asked, Kind, Path, Regex, bound, search};

const STD_TYPES: &[&str] = &[
    "string",
    "number",
    "boolean",
    "bigint",
    "Array",
    "ReadonlyArray",
    "Map",
    "ReadonlyMap",
    "Set",
    "ReadonlySet",
    "Promise",
];

const STD_MEMBERS: &[&str] = &[
    "length",
    "at",
    "concat",
    "copyWithin",
    "entries",
    "every",
    "fill",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "flat",
    "flatMap",
    "forEach",
    "includes",
    "indexOf",
    "join",
    "keys",
    "lastIndexOf",
    "map",
    "pop",
    "push",
    "reduce",
    "reduceRight",
    "reverse",
    "shift",
    "slice",
    "some",
    "sort",
    "splice",
    "toReversed",
    "toSorted",
    "toSpliced",
    "unshift",
    "values",
    "with",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "endsWith",
    "localeCompare",
    "match",
    "matchAll",
    "normalize",
    "padEnd",
    "padStart",
    "repeat",
    "replace",
    "replaceAll",
    "search",
    "split",
    "startsWith",
    "substring",
    "substr",
    "toLowerCase",
    "toUpperCase",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "trim",
    "trimEnd",
    "trimStart",
    "toString",
    "toLocaleString",
    "valueOf",
    "size",
    "get",
    "set",
    "has",
    "delete",
    "clear",
    "add",
    "then",
    "catch",
    "finally",
    "toFixed",
    "toPrecision",
    "toExponential",
    "hasOwnProperty",
];

const STD_RESULTS: &[&str] = &[
    "join",
    "split",
    "trim",
    "trimStart",
    "trimEnd",
    "padStart",
    "padEnd",
    "toLowerCase",
    "toUpperCase",
    "toFixed",
    "repeat",
    "replace",
    "replaceAll",
    "substring",
    "map",
    "filter",
    "flat",
    "flatMap",
    "then",
    "catch",
    "finally",
];

fn std_global_call(global: &str, method: &str) -> bool {
    match global {
        "Promise" => matches!(
            method,
            "resolve" | "reject" | "all" | "allSettled" | "any" | "race"
        ),
        "Array" => matches!(method, "from" | "of"),
        "Object" => matches!(method, "keys" | "values" | "entries"),
        "JSON" => method == "stringify",
        _ => false,
    }
}

fn std_type(written: &str) -> Option<Vec<String>> {
    let mut members = vec![String::new()];
    let mut depth = 0i32;
    let mut last = ' ';
    for c in written.chars() {
        match c {
            '<' | '(' | '[' | '{' => depth += 1,
            '>' if last == '=' => {}
            '>' | ')' | ']' | '}' => depth -= 1,
            '|' | '&' if depth == 0 => {
                members.push(String::new());
                last = c;
                continue;
            }
            _ => {}
        }
        members.last_mut()?.push(c);
        last = c;
    }
    let bases: Vec<String> = members
        .iter()
        .map(|m| m.trim())
        .filter(|m| !m.is_empty() && *m != "null" && *m != "undefined")
        .map(|t| {
            if t.ends_with("[]") {
                return Some(t.to_owned());
            }
            let base = t.split('<').next()?.trim();
            STD_TYPES.contains(&base).then(|| base.to_owned())
        })
        .collect::<Option<_>>()?;
    (depth == 0 && !bases.is_empty()).then_some(bases)
}

fn receiver_text(before: &str) -> &str {
    let mut depth = 0usize;
    let mut quote = None;
    for (i, c) in before.char_indices().rev() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            ')' | ']' => depth += 1,
            '(' | '[' if depth == 0 => return before.get(i + 1..).unwrap_or_default(),
            '(' | '[' => depth -= 1,
            '"' | '\'' | '`' if depth > 0 => quote = Some(c),
            _ if depth > 0 => {}
            c if c.is_alphanumeric() || matches!(c, '_' | '$' | '.' | '?' | '!') => {}
            _ => return before.get(i + c.len_utf8()..).unwrap_or_default(),
        }
    }
    before
}

fn last_call(expr: &str) -> Option<(&str, &str)> {
    let inner = expr.strip_suffix(')')?;
    let mut depth = 0usize;
    let mut quote = None;
    for (i, c) in inner.char_indices().rev() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' | '`' => quote = Some(c),
            ')' => depth += 1,
            '(' if depth == 0 => {
                let callee = inner.get(..i)?.trim_end().trim_end_matches('?');
                return callee.rsplit_once('.');
            }
            '(' => depth -= 1,
            _ => {}
        }
    }
    None
}

impl App {
    pub(super) fn js_std_receiver(&self, a: &Asked) -> Option<String> {
        if a.kind != Kind::TsJs || !STD_MEMBERS.contains(&a.word.as_str()) {
            return None;
        }
        let before = a.written.get(..a.start)?.strip_suffix('.')?;
        let before = before.strip_suffix('?').unwrap_or(before);
        let text = self.buf.lines.join("\n");
        let mut links = Vec::new();
        self.js_std(
            &a.here,
            &text,
            self.line + 1,
            receiver_text(before),
            0,
            &mut links,
        )?;
        Some(links.join(" \u{2192} "))
    }

    fn js_std(
        &self,
        here: &Path,
        text: &str,
        line1: usize,
        expr: &str,
        depth: usize,
        links: &mut Vec<String>,
    ) -> Option<()> {
        let kind = Kind::TsJs;
        let expr = expr.trim().trim_end_matches(['!', ';']).trim_end();
        if depth > 4 {
            return None;
        }
        if let Some((receiver, method)) = last_call(expr) {
            let receiver = receiver.trim_end_matches('?');
            if std_global_call(receiver, method)
                && self.js_global_unshadowed(here, text, line1, receiver)
            {
                links.push(format!("{receiver}.{method}()"));
                return Some(());
            }
            if !STD_RESULTS.contains(&method) || receiver.is_empty() {
                return None;
            }
            if self
                .js_std(here, text, line1, receiver, depth + 1, links)
                .is_none()
                && self.declared_in_project(here, method)
            {
                return None;
            }
            links.push(format!("{method}()"));
            return Some(());
        }
        if !expr
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
            || expr.is_empty()
        {
            return None;
        }
        let bindings = search::bindings(kind, text, line1, expr);
        let imports = search::imports(kind, text);
        let [b] = bindings.as_slice() else {
            return None;
        };
        if let search::Value::Type(written) = &b.value {
            if std_type(written)?.iter().any(|base| {
                bound(&imports, base).is_some() || self.type_decl(kind, here, base).is_some()
            }) {
                return None;
            }
            links.push(format!("{expr}: {}", written.trim()));
            return Some(());
        }
        let line = text.lines().nth(b.line1.checked_sub(1)?)?;
        let assigned = Regex::new(&format!(
            r"\b(?:const|let|var)\s+{}\s*=\s*(.+)$",
            regex::escape(expr)
        ))
        .ok()?;
        let value = assigned.captures(line)?.get(1)?.as_str();
        self.js_std(here, text, b.line1, value, depth + 1, links)
    }

    fn js_global_unshadowed(&self, here: &Path, text: &str, line1: usize, global: &str) -> bool {
        let kind = Kind::TsJs;
        search::bindings(kind, text, line1, global).is_empty()
            && bound(&search::imports(kind, text), global).is_none()
            && !self.declared_in_project(here, global)
    }

    fn declared_in_project(&self, here: &Path, word: &str) -> bool {
        let kind = Kind::TsJs;
        let mut patterns = search::def_patterns(kind, word);
        patterns.extend(search::member_or_signature(kind, word).unwrap_or_default());
        let cut = self.truncated.get();
        let found = !self
            .project_definitions(kind, here, word, &patterns.join("|"))
            .is_empty();
        self.truncated.set(cut);
        found
    }
}
