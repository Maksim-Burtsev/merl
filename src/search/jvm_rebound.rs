use regex::Regex;

use super::jvm::{anonymous, ident, jvm_bindings, param_lists_line1_binding, param_names};
use super::{Binding, Kind, Value, code, indent, uncommented};

pub(super) fn blanked(s: &str) -> String {
    let mut bytes = vec![b' '; s.len()];
    for (i, c) in code(Kind::Jvm, s) {
        bytes[i] = if c.is_ascii() { c } else { b' ' };
    }
    String::from_utf8(bytes).unwrap_or_default()
}

pub fn jvm_rebound_before(before: &str, name: &str) -> bool {
    let n = regex::escape(name);
    let ret = super::jvm_return_type!();
    let Ok(declared) = Regex::new(&format!(
        r"[;{{]\s*(?:final\s+)?(?:val|var|{ret})\s+{n}\s*[=;:,]|[(,]\s*(?:final\s+)?{ret}\s+{n}\s*[,)]|[(,]\s*{n}\s*:\s*[A-Z]"
    )) else {
        return false;
    };
    let line = blanked(before);
    declared.find_iter(&line).any(|m| {
        let mut depth = 0i32;
        line.get(m.end()..).is_some_and(|rest| {
            rest.bytes().all(|b| {
                match b {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                depth >= 0
            })
        })
    })
}

fn later_declarator(code: &str, name: &str) -> Option<(String, usize)> {
    static HEAD: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(concat!(
            r"^\s*(?:(?:final|@[\w.]+)\s+)*(",
            super::jvm_return_type!(),
            r")\s+[A-Za-z_]\w*\s*(?:=[^=]|,)"
        ))
        .expect("the regex is a literal")
    });
    let head = HEAD.captures(code)?;
    let mut depth = 0i32;
    for (i, b) in code.bytes().enumerate().skip(head.get(0)?.end() - 1) {
        match b {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' | b';' if depth == 0 => return None,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                let segment = code.get(i + 1..)?;
                let at = i + 1 + segment.len() - segment.trim_start().len();
                let rest = code.get(at..)?;
                let Some(after) = rest.strip_prefix(name) else {
                    let init = rest
                        .trim_start_matches(|c: char| c.is_alphanumeric() || c == '_')
                        .trim_start();
                    if init.starts_with('=') && !init.starts_with("==") {
                        continue;
                    }
                    return None;
                };
                let tail = after.trim_start();
                if !after.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                    && (tail.is_empty()
                        || tail.starts_with([',', ';'])
                        || tail.starts_with('=') && !tail.starts_with("=="))
                {
                    return Some((head[1].to_owned(), at));
                }
            }
            _ => {}
        }
    }
    None
}

static FOR: std::sync::LazyLock<Regex> =
    std::sync::LazyLock::new(|| Regex::new(r"\bfor\s*\(").expect("the regex is a literal"));

pub(super) fn later_declarator_type(line: &str, name: &str) -> Option<String> {
    let line = blanked(line);
    std::iter::once(0)
        .chain(line.match_indices([';', '{', '}']).map(|(i, _)| i + 1))
        .chain(FOR.find_iter(&line).map(|m| m.end()))
        .find_map(|s| later_declarator(line.get(s..)?, name).map(|(t, _)| t))
}

pub(super) fn for_init_declarators(line: &str, name: &str) -> Vec<(usize, usize)> {
    FOR.find_iter(line)
        .filter_map(|m| {
            let (_, at) = later_declarator(line.get(m.end()..)?, name)?;
            Some((m.start(), m.end() + at))
        })
        .collect()
}

pub(super) fn continued_declarator(
    lines: &[&str],
    i: usize,
    before: usize,
    name: &str,
) -> Option<usize> {
    let head = blanked(&uncommented(Kind::Jvm, lines[i]));
    if !head.trim_end().ends_with(',') {
        return None;
    }
    let mut joined = head.trim_start().to_owned();
    let mut starts = vec![(0, i)];
    for k in (i + 1..before)
        .take_while(|&k| lines[k].trim().is_empty() || indent(lines[k]) > indent(lines[i]))
    {
        joined.push(' ');
        starts.push((joined.len(), k));
        joined.push_str(&blanked(&uncommented(Kind::Jvm, lines[k])));
    }
    let (_, at) = later_declarator(&joined, name)?;
    starts.iter().rev().find(|(s, _)| *s <= at).map(|&(_, k)| k)
}

pub(super) fn case_label_end(t: &str) -> usize {
    static LABEL: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:case\b[^;{}:]*|default)\s*:").expect("the regex is a literal")
    });
    LABEL.find(t).map_or(0, |m| m.end())
}

pub(super) fn declares_local(t: &str, name: &str) -> bool {
    let t = t.get(case_label_end(t)..).unwrap_or(t).trim_start();
    let n = regex::escape(name);
    let ret = super::jvm_return_type!();
    Regex::new(&format!(
        r"^(?:(?:final|lateinit|const)\s+)*(?:(?:val|var|def|fun)\s+{n}(?:[^\w.]|$)|(?:val|var|def)\s*\([^)]*\b{n}\b[^)]*\)\s*=|(?:@[\w.]+\s+)*(?:var|{ret})(?:\.\.\.)?\s+{n}\s*[=;,:])"
    ))
    .is_ok_and(|re| re.is_match(t))
        || later_declarator(&blanked(t), name).is_some()
}

pub(super) fn local_def_header(lines: &[&str], i: usize, name: &str, keyword: &str) -> bool {
    let header = uncommented(Kind::Jvm, lines[i]);
    header.trim_start().starts_with(keyword)
        && declares_local(header.trim(), name)
        && super::jvm::scope_of(lines, i).is_some_and(|s| !super::jvm::opens_type(lines[s]))
}

pub(super) fn jvm_declared_before(lead: &str, name: &str) -> bool {
    let n = regex::escape(name);
    let ret = super::jvm_return_type!();
    let Ok(declared) = Regex::new(&format!(
        r"(?:^|[;{{]|\b(?:case\b[^;{{}}:]*|default)\s*:)\s*(?:final\s+)?(?:(?:val|var|def|{ret})\s+{n}\s*(?:=[^=]|[;:,])|(?:def|fun)\s+{n}\s*(?:\[[^\]]*\])?(?:\([^()]*\))+\s*(?::[^=;{{}}()]*)?=[^=]|(?:val|var|def)\s*\([^)]*\b{n}\b[^)]*\)\s*=[^=])"
    )) else {
        return false;
    };
    let later = std::iter::once(0)
        .chain(lead.match_indices([';', '{']).map(|(i, _)| i + 1))
        .filter_map(|s| later_declarator(lead.get(s..)?, name).map(|(_, at)| s + at + name.len()));
    declared
        .find_iter(lead)
        .map(|m| m.end())
        .chain(later)
        .any(|end| {
            let (mut depth, mut ended) = (0i32, false);
            lead.get(end..).is_some_and(|rest| {
                rest.bytes().all(|b| {
                    match b {
                        b'(' | b'[' | b'{' => depth += 1,
                        b')' | b']' | b'}' => depth -= 1,
                        b';' if depth == 0 => ended = true,
                        _ => {}
                    }
                    depth >= 0
                })
            }) && ended
        })
}

pub(super) fn negated_pattern_exits(lines: &[&str], at: usize, name: &str) -> bool {
    static EXIT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\b(?:return|throw|continue|break)\b").expect("the regex is a literal")
    });
    let Ok(pattern) = Regex::new(&format!(
        r"!\s*\(.*\binstanceof\s+(?:final\s+)?[\w.]+(?:<[^>]*>)?\s+{}\b",
        regex::escape(name)
    )) else {
        return false;
    };
    if !pattern.is_match(&blanked(&uncommented(Kind::Jvm, lines[at]))) {
        return false;
    }
    let depth = indent(lines[at]);
    std::iter::once(lines[at])
        .chain(
            lines[at + 1..]
                .iter()
                .copied()
                .take_while(|l| l.trim().is_empty() || indent(l) > depth),
        )
        .any(|l| EXIT.is_match(&blanked(&uncommented(Kind::Jvm, l))))
}

pub(super) fn lambda_names(line: &str, scala: bool) -> Vec<String> {
    let code = blanked(line);
    lambda_binders(line, scala)
        .into_iter()
        .filter(|(_, at)| {
            let opener = line.get(at.clone());
            !opener.is_some_and(|o| o.trim_start().starts_with("return"))
                || code.get(at.clone()) == opener && lambda_open_at(&code, at, code.len())
        })
        .map(|(n, _)| n)
        .collect()
}

pub(super) fn lambda_binders(line: &str, scala: bool) -> Vec<(String, std::ops::Range<usize>)> {
    static SCALA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[(,{=]|=>)\s*(?:\(([^()]*)\)|([A-Za-z_]\w*))\s*=>")
            .expect("the regex is a literal")
    });
    static CASE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bcase\s+(.+?)\s*(?:\bif\b.*?)?=>").expect("the regex is a literal")
    });
    static BINDER: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?:^|[^\w.`"'])([a-z_]\w*)\s*([(.]?)"#).expect("the regex is a literal")
    });
    static KOTLIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\{\s*([^{}]*?)\s*->").expect("the regex is a literal")
    });
    static JAVA: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?:^|[(,=]|->|\breturn\b)\s*(?:\(([^()]*)\)|([A-Za-z_]\w*))\s*->")
            .expect("the regex is a literal")
    });
    static GROOVY_TYPED: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:final\s+)?(?:def|int|long|short|byte|char|boolean|float|double|[A-Z][\w.]*(?:<[^<>]*>)?(?:\[\])*)\s+([A-Za-z_]\w*)$")
            .expect("the regex is a literal")
    });
    let mut out = Vec::new();
    for c in KOTLIN.captures_iter(line) {
        let at = c.get(0).map_or(0..0, |m| m.range());
        for entry in c[1].split(',') {
            let e = entry.trim().trim_matches(['(', ')']).trim();
            let name = e.split(':').next().unwrap_or("").trim();
            if ident(name) {
                out.push((name.to_owned(), at.clone()));
            } else if let Some(typed) = GROOVY_TYPED.captures(e) {
                out.push((typed[1].to_owned(), at.clone()));
            }
        }
    }
    for c in CASE.captures_iter(line) {
        let at = c.get(0).map_or(0..0, |m| m.range());
        for b in BINDER.captures_iter(&c[1]).filter(|b| b[2].is_empty()) {
            out.push((b[1].to_owned(), at.clone()));
        }
    }
    let java = JAVA
        .captures_iter(line)
        .filter(|c| !scala || !c[0].trim_start().starts_with("return"));
    for c in java.chain(SCALA.captures_iter(line)) {
        let at = c.get(0).map_or(0..0, |m| m.range());
        match (c.get(1), c.get(2)) {
            (Some(list), _) => {
                for entry in list.as_str().split(',') {
                    let e = entry.trim();
                    let names = if e.contains(char::is_whitespace) {
                        param_names(e, false)
                    } else {
                        ident(e).then(|| e.to_owned()).into_iter().collect()
                    };
                    out.extend(names.into_iter().map(|n| (n, at.clone())));
                }
            }
            (None, Some(name)) => out.push((name.as_str().to_owned(), at)),
            _ => {}
        }
    }
    out.retain(|(n, _)| n != "_");
    out
}

pub(super) fn lambda_open_at(line: &str, at: &std::ops::Range<usize>, col: usize) -> bool {
    if col < at.start {
        return false;
    }
    let case = line.get(at.start..).is_some_and(|l| l.starts_with("case"));
    let braced = case || line.get(at.start..).is_some_and(|l| l.starts_with('{'));
    let mut depth = 0i32;
    line.get(at.end.min(col)..col).is_some_and(|body| {
        let word = |i: usize| {
            body.as_bytes()
                .get(i)
                .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        };
        body.bytes().enumerate().all(|(i, b)| {
            match b {
                b'(' | b'[' | b'{' => depth += 1,
                b')' | b']' | b'}' => depth -= 1,
                _ => {}
            }
            let next_case = case
                && depth == 0
                && body.get(i..).is_some_and(|r| r.starts_with("case"))
                && !word(i.wrapping_sub(1))
                && !word(i + 4);
            depth >= 0 && !next_case && (braced || depth > 0 || !matches!(b, b';' | b','))
        })
    })
}

pub fn jvm_anonymous_open(before: &str) -> bool {
    jvm_anonymous_header(before).is_some()
}

pub fn jvm_anonymous_header(before: &str) -> Option<String> {
    static OPENS: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bnew\s+[\w.]+\s*(?:<[^{}]*>|\[[^\]{}]+\])?\s*(?:\([^{}]*\))?(?:\s+with\s+[\w.]+)*\s*\{|\bobject\s*:[^{}]*\{")
            .expect("the regex is a literal")
    });
    let line = blanked(before);
    let open = OPENS.find_iter(&line).filter(|m| {
        let mut depth = 0i32;
        line.get(m.end()..).is_some_and(|rest| {
            rest.bytes().all(|b| {
                match b {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                depth >= 0
            })
        })
    });
    open.last()
        .and_then(|m| line.get(..m.end()).map(str::to_owned))
}

pub(super) fn clause_closed_at(line: &str, start: usize, col: usize) -> bool {
    let line = &without_block_comments(line);
    let b = line.as_bytes();
    let rest = line.get(start..).unwrap_or_default();
    if line.get(..col).is_none() {
        return false;
    }
    if rest.starts_with("case") {
        return case_closed_at(b, start, col);
    }
    let open = if rest.starts_with("instanceof") {
        match pattern_header(line, start) {
            Header::If(o) => o,
            Header::Statement(from) => {
                return statement_end(b, from, start).is_some_and(|e| e <= col);
            }
            Header::Other => return false,
        }
    } else if ["for", "catch", "try"].iter().any(|k| rest.starts_with(k)) {
        match rest.find('(') {
            Some(o) => start + o,
            None => return false,
        }
    } else {
        return false;
    };
    let Some(close) = matching(b, open) else {
        return false;
    };
    let instanceof = rest.starts_with("instanceof");
    if instanceof
        && line
            .get(open..close)
            .is_none_or(|h| h.contains(['!', '=', '^', '?']))
    {
        return false;
    }
    let Some(end) = body_end(line, close + 1).filter(|&e| e <= col) else {
        return false;
    };
    if !instanceof {
        return true;
    }
    match line
        .get(end..)
        .unwrap_or_default()
        .trim_start()
        .strip_prefix("else")
        .filter(|r| !r.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    {
        Some(r) => {
            let from = line.len() - r.len();
            body_end(line, from).is_some_and(|e| {
                col < e || line.get(from..e).is_some_and(|body| !LEAVES.is_match(body))
            })
        }
        None => true,
    }
}

static LEAVES: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"\b(?:return|throw|break|continue|yield|while|for|do)\b")
        .expect("the regex is a literal")
});

fn without_block_comments(line: &str) -> String {
    let mut out = line.to_owned();
    let mut from = 0;
    while let Some(open) = out.get(from..).and_then(|r| r.find("/*")).map(|o| from + o) {
        let close = out
            .get(open + 2..)
            .and_then(|r| r.find("*/"))
            .map_or(out.len(), |c| open + 2 + c + 2);
        out.replace_range(open..close, &" ".repeat(close - open));
        from = close;
    }
    out
}

enum Header {
    If(usize),
    Statement(usize),
    Other,
}

fn pattern_header(line: &str, start: usize) -> Header {
    static DECLARATION: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:return\b|(?:final\s+)?[\w.]+(?:<[^<>]*>)?(?:\[\])*\s+\w+\s*=[^=])")
            .expect("the regex is a literal")
    });
    let b = line.as_bytes();
    let (mut depth, mut innermost) = (0i32, true);
    for i in (0..start).rev() {
        match b[i] {
            b')' | b']' | b'}' => depth += 1,
            b'(' | b'[' | b'{' if depth > 0 => depth -= 1,
            b'{' => return Header::Statement(i + 1),
            b'(' => {
                let head = line.get(..i).unwrap_or_default().trim_end();
                let word = head
                    .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .next()
                    .unwrap_or_default();
                match word {
                    "if" if innermost => return Header::If(i),
                    "if" | "while" | "for" | "switch" | "synchronized" | "catch" | "try" => {
                        return Header::Other;
                    }
                    _ => innermost = false,
                }
            }
            _ => {}
        }
    }
    if depth == 0 && DECLARATION.is_match(line) {
        Header::Statement(0)
    } else {
        Header::Other
    }
}

fn statement_end(b: &[u8], from: usize, start: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, &c) in b.iter().enumerate().skip(from) {
        match c {
            b'(' | b'[' | b'{' => depth += 1,
            b'}' if depth == 0 => return Some(i),
            b')' | b']' if depth == 0 => return None,
            b')' | b']' | b'}' => depth -= 1,
            b';' if depth == 0 && i > start => return Some(i + 1),
            _ => {}
        }
    }
    None
}

fn case_closed_at(b: &[u8], start: usize, col: usize) -> bool {
    let word = |i: usize| {
        b.get(i)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
    };
    let mut depth = 0i32;
    for i in start + 4..col.min(b.len()) {
        match b[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b'}' if depth == 0 => return true,
            b')' | b']' if depth == 0 => return false,
            b')' | b']' | b'}' => depth -= 1,
            _ if depth == 0
                && !word(i.wrapping_sub(1))
                && [&b"case"[..], b"default"]
                    .iter()
                    .any(|k| b[i..].starts_with(k) && !word(i + k.len())) =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn body_end(line: &str, from: usize) -> Option<usize> {
    static STATEMENT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*(?:if|for|while|do|try|switch|synchronized|else)\b")
            .expect("the regex is a literal")
    });
    let b = line.as_bytes();
    let after = line.get(from..).unwrap_or_default();
    let body = line.len() - after.trim_start().len();
    if b.get(body) == Some(&b'{') {
        matching(b, body).map(|e| e + 1)
    } else if STATEMENT.is_match(after) {
        None
    } else {
        let mut depth = 0i32;
        (body..b.len())
            .find(|&i| {
                match b[i] {
                    b'(' | b'[' | b'{' => depth += 1,
                    b')' | b']' | b'}' => depth -= 1,
                    _ => {}
                }
                depth < 0 || (depth == 0 && b[i] == b';')
            })
            .map(|e| e + 1)
    }
}

fn matching(b: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    (open..b.len()).find(|&i| {
        match b[i] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            _ => {}
        }
        depth == 0
    })
}

pub fn jvm_bindings_at(
    path: &std::path::Path,
    text: &str,
    line1: usize,
    col: usize,
    name: &str,
) -> Vec<Binding> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(at) = line1.checked_sub(1).filter(|&i| i < lines.len()) else {
        return Vec::new();
    };
    if !matches!(name, "this" | "super" | "it")
        && blanked(lines[at])
            .get(..col)
            .is_some_and(|lead| jvm_declared_before(lead, name))
    {
        return vec![Binding {
            line1,
            value: Value::Unknown,
        }];
    }
    jvm_bindings(path, &lines, at, name, Some(col))
}

pub fn jvm_anonymous_bases(line: &str) -> Vec<String> {
    static BASE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bobject\s*:\s*([\w.]+)|\bnew\s+([\w.]+)").expect("the regex is a literal")
    });
    static MIXIN: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"\bwith\s+([\w.]+)").expect("the regex is a literal")
    });
    if !anonymous(line) {
        return Vec::new();
    }
    let Some(c) = BASE.captures_iter(line).last() else {
        return Vec::new();
    };
    let Some(first) = c.get(1).or_else(|| c.get(2)) else {
        return Vec::new();
    };
    std::iter::once(first.as_str())
        .chain(
            MIXIN
                .captures_iter(line.get(first.end()..).unwrap_or_default())
                .filter_map(|m| m.get(1).map(|w| w.as_str())),
        )
        .map(str::to_owned)
        .collect()
}

pub(super) fn header_lines1_binding(
    lines: &[&str],
    from: usize,
    to: usize,
    name: &str,
    cursor: Option<usize>,
    scala: bool,
) -> Vec<usize> {
    let n = regex::escape(name);
    let rule = |p: String| Regex::new(&p).expect("an escaped name keeps the pattern valid");
    let ret = super::jvm_return_type!();
    let clause = rule(format!(
        r"\bfor\s*\(\s*(?:(?:final\s+)?(?:var|val|{ret})\s+)?{n}\s*(?:[:=]|<-|\s+in\b)|\bfor\s*\(\s*[\w(),\s]*?<-[^{{}}]*?;\s*{n}\s*(?::[^<;]*)?(?:<-|=[^=>])|\bfor\s*\(\s*\([^)]*\b{n}\b[^)]*\)\s+in\b|\bcatch\s*\(\s*(?:final\s+)?[\w.|\s]+?\s+{n}\s*\)|\bcatch\s*\(\s*{n}\s*:|\btry\s*\(\s*(?:final\s+)?(?:var|{ret})\s+{n}\s*=|\b(?:instanceof|case)\s+(?:final\s+)?[\w.]+(?:<[^>]*>)?\s+{n}\b|\b(?:instanceof|case)\s+[\w.]+(?:<[^>]*>)?\s*\([^{{}};]*\b(?:var|{ret})\s+{n}\s*[,)]"
    ));
    let mut out: Vec<usize> = param_lists_line1_binding(lines, from, name)
        .into_iter()
        .collect();
    for (i, l) in lines.iter().enumerate().take(to + 1).skip(from) {
        let code = uncommented(Kind::Jvm, l);
        let (lambda, clause) = match cursor {
            Some(col) => {
                let line = blanked(l);
                let lambda = lambda_binders(&line, scala)
                    .iter()
                    .any(|(p, at)| p == name && lambda_open_at(&line, at, col));
                let open = std::iter::successors(clause.find(&line), |m| {
                    clause.find_at(&line, m.start() + 1)
                })
                .any(|m| !clause_closed_at(&line, m.start(), col))
                    || for_init_declarators(&line, name)
                        .iter()
                        .any(|&(s, at)| at <= col && !clause_closed_at(&line, s, col));
                (lambda, open)
            }
            None => (
                lambda_names(&code, scala).iter().any(|p| p == name),
                clause.is_match(&code) || !for_init_declarators(&blanked(&code), name).is_empty(),
            ),
        };
        if clause || lambda {
            out.push(i + 1);
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}
