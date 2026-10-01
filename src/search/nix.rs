use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::*;

const NAME: &str = r"[A-Za-z_][\w'-]*";

pub fn nix_patterns(word: &str) -> Vec<String> {
    let w = regex::escape(word);
    let part = format!(r#"(?:{NAME}|"[^"]*")"#);
    vec![
        format!(r#"(?:^|[{{;]|\blet\b)\s*(?:{part}\s*\.\s*)*(?:{w}|"{w}")\s*=(?:[^=]|$)"#),
        format!(r"\binherit\b(?:\s*\([^)]*\))?(?:\s+[\w'-]+)*?\s+{w}(?:\s+[\w'-]+)*\s*(?:;|$)"),
    ]
}

pub(super) const NIX_FUNCTION_SYMBOL: &str = concat!(
    r"^\s*(?:let\s+)?(?P<name>[A-Za-z_][\w'-]*)\s*=\s*",
    r"(?:[A-Za-z_][\w'-]*\s*:(?:\s|$)",
    r"|(?:[A-Za-z_][\w'-]*\s*@\s*)?\{[^{}]*\}\s*(?:@\s*[A-Za-z_][\w'-]*\s*)?:(?:\s|$))"
);

pub fn nix_name(line: &str, r: Range<usize>) -> Range<usize> {
    let primes = line.as_bytes()[r.end..]
        .iter()
        .take_while(|&&c| c == b'\'')
        .count();
    r.start..r.end + usize::from(primes == 1)
}

pub fn nix_path(line: &str, col: usize) -> Option<String> {
    static PATH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[\s(\[{=;])((?:\.{1,2}|[\w.+-]*)(?:/[\w.+-]+)+/?)").unwrap()
    });
    let m = PATH
        .captures_iter(line)
        .filter_map(|c| c.get(1))
        .find(|m| m.start() <= col && col < m.end())?;
    let quotes = line[..m.start()].matches('"').count() - line[..m.start()].matches("\\\"").count();
    quotes.is_multiple_of(2).then(|| m.as_str().trim_end_matches('/').to_owned())
}

pub fn nix_files(dir: &Path, path: &str, files: &[PathBuf]) -> Vec<PathBuf> {
    let Some(at) = lexical(&dir.join(path)) else {
        return Vec::new();
    };
    [at.clone(), at.join("default.nix")]
        .into_iter()
        .find(|f| files.contains(f))
        .into_iter()
        .collect()
}

fn code_of(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let (mut quoted, mut escaped) = (false, false);
    for c in line.chars() {
        if quoted {
            out.push(if c == '"' && !escaped { '"' } else { ' ' });
            quoted = escaped || c != '"';
            escaped = !escaped && c == '\\';
        } else if c == '#' {
            break;
        } else {
            quoted = c == '"';
            out.push(c);
        }
    }
    out
}

fn opens_let(t: &str) -> bool {
    t == "let" || t.starts_with("let ") || t.ends_with(" let") || t.ends_with("=let")
}

fn starts_in(t: &str) -> bool {
    t == "in" || t.starts_with("in ") || t.starts_with("in{")
}

fn owner<S: AsRef<str>>(lines: &[S], i: usize) -> Option<usize> {
    let depth = indent(lines[i].as_ref());
    (0..i).rev().find(|&j| {
        let t = lines[j].as_ref().trim();
        !t.is_empty() && !t.starts_with('#') && indent(lines[j].as_ref()) < depth
    })
}

pub fn nix_let_bound<S: AsRef<str>>(lines: &[S], line: usize) -> bool {
    let Some(i) = line.checked_sub(1).filter(|&i| i < lines.len()) else {
        return false;
    };
    lines[i].as_ref().trim_start().starts_with("let ")
        || owner(lines, i).is_some_and(|o| opens_let(code_of(lines[o].as_ref()).trim()))
}

struct Scope<'a> {
    lines: &'a [&'a str],
    code: Vec<String>,
    skip: Vec<bool>,
    binds: Regex,
    inherits: Regex,
    name: &'a str,
}

impl Scope<'_> {
    fn binds_here(&self, i: usize) -> bool {
        self.binds.is_match(&self.code[i]) || self.inherits.is_match(&self.code[i])
    }

    fn let_binding(&self, j: usize) -> Option<usize> {
        let depth = indent(self.lines[j]);
        let end = (j + 1..self.lines.len())
            .find(|&k| !self.skip[k] && indent(self.lines[k]) <= depth)
            .unwrap_or(self.lines.len());
        let own = self.code[j].trim_start().starts_with("let ") && self.binds_here(j);
        if own {
            return Some(j);
        }
        (j + 1..end)
            .find(|&k| !self.skip[k] && owner(self.lines, k) == Some(j) && self.binds_here(k))
    }

    fn set_pattern(&self, j: usize, from: usize) -> Option<(usize, String, String)> {
        static END: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(&format!(r"^\s*(?:@\s*({NAME})\s*)?:(?:\s|$)")).unwrap());
        let (mut depth, mut inner) = (0usize, String::new());
        for k in j..self.lines.len().min(j + 200) {
            let code = &self.code[k][if k == j { from } else { 0 }..];
            for (n, c) in code.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            let c = END.captures(&code[n + 1..])?;
                            let alias = c.get(1).map_or("", |a| a.as_str()).to_owned();
                            return Some((k, inner, alias));
                        }
                    }
                    _ => {}
                }
                if depth > 1 || (depth == 1 && c != '{') {
                    inner.push(c);
                }
            }
            inner.push(',');
        }
        None
    }

    fn param(&self, j: usize, at_start: bool) -> Option<usize> {
        static PLAIN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(&format!(r"(?:^|[\s(=;:])({NAME})\s*:(?:\s|$)")).unwrap());
        static ALIAS: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(&format!(r"({NAME})\s*@\s*$")).unwrap());
        let code = &self.code[j];
        let lead = indent(code);
        let plain = PLAIN.captures_iter(code).filter_map(|c| c.get(1));
        if plain
            .filter(|m| !at_start || m.start() == lead)
            .any(|m| m.as_str() == self.name)
        {
            return Some(j);
        }
        for (p, _) in code.match_indices('{') {
            let alias = ALIAS.captures(&code[..p]).and_then(|c| c.get(1));
            let start = alias.map_or(p, |a| a.start());
            if at_start && start != lead {
                continue;
            }
            let Some((end, inner, after)) = self.set_pattern(j, p) else {
                continue;
            };
            let named = |n: &str| n.split('?').next().is_some_and(|n| n.trim() == self.name);
            if alias.is_some_and(|a| a.as_str() == self.name) || after == self.name {
                return Some(j);
            }
            if inner.split(',').any(named) {
                return (j..=end).find(|&k| names(&self.code[k], self.name));
            }
        }
        None
    }
}

pub(super) fn nix_bindings(lines: &[&str], at: usize, name: &str) -> Vec<Binding> {
    let n = regex::escape(name);
    let literal = literal_lines(Kind::Nix, &lines.join("\n"));
    let scope = Scope {
        lines,
        code: lines.iter().map(|l| code_of(l)).collect(),
        skip: (0..lines.len())
            .map(|i| literal[i] || lines[i].trim().is_empty() || lines[i].trim().starts_with('#'))
            .collect(),
        binds: Regex::new(&format!(r#"^\s*(?:let\s+)?(?:{n}|"{n}")\s*=(?:[^=]|$)"#)).unwrap(),
        inherits: Regex::new(&format!(
            r"^\s*(?:let\s+)?inherit\b(?:\s*\([^)]*\))?(?:\s+[\w'-]+)*?\s+{n}(?:\s+[\w'-]+)*\s*(?:;|$)"
        ))
        .unwrap(),
        name,
    };
    let found = |line: usize| {
        vec![Binding {
            line: line + 1,
            value: Value::Unknown,
        }]
    };
    if (scope.binds_here(at) && nix_let_bound(lines, at + 1)) || scope.param(at, false).is_some() {
        return found(scope.param(at, false).unwrap_or(at));
    }
    let mut depth = indent(lines[at]);
    let mut i = at + 1;
    while i > 0 {
        i -= 1;
        let t = scope.code[i].trim();
        let ind = indent(lines[i]);
        if (i != at && scope.skip[i]) || ind > depth {
            continue;
        }
        if starts_in(t) {
            let Some(j) = (0..i).rev().find(|&j| {
                !scope.skip[j] && indent(lines[j]) <= ind && opens_let(scope.code[j].trim())
            }) else {
                continue;
            };
            if let Some(b) = scope.let_binding(j) {
                return found(b);
            }
            (i, depth) = (j + 1, depth.min(indent(lines[j])));
            continue;
        }
        if i == at {
            continue;
        }
        if ind < depth {
            depth = ind;
            let hit = match opens_let(t) {
                true => scope.let_binding(i),
                false => scope.param(i, false),
            };
            if let Some(b) = hit {
                return found(b);
            }
        } else if let Some(b) = scope.param(i, true) {
            return found(b);
        }
    }
    Vec::new()
}
