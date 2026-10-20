use std::path::Path;

use regex::Regex;

use super::{Hit, SQL_NAME_PART, sql_create};

pub fn sql_cte(word: &str) -> String {
    let w = regex::escape(word);
    format!(
        r"(?i)^\s*(?:\)\s*)?(?:WITH\s+(?:RECURSIVE\s+)?|,\s*)?{w}(?:\s*\([^()]*\))?\s+AS(?:\s+(?:NOT\s+)?MATERIALIZED)?\s*\("
    )
}
pub fn sql_cte_sees(text: &str, cte: usize, word: &str, line: usize, col: usize) -> Option<bool> {
    let start = |l: usize| -> usize { text.split_inclusive('\n').take(l - 1).map(str::len).sum() };
    let head = Regex::new(&sql_cte(word)).expect("an escaped name keeps the pattern valid");
    let Some(m) = text.lines().nth(cte - 1).and_then(|l| head.find(l)) else {
        return Some(false);
    };
    let (open, at, b) = (start(cte) + m.end() - 1, start(line) + col, text.as_bytes());
    let (mut first, mut depth, mut close, mut i) = (0, None::<usize>, None, 0);
    let (mut ended, mut after) = (false, 0_usize);
    while i < b.len() {
        let skip = match &b[i..] {
            [b'-', b'-', ..] => "\n",
            [b'/', b'*', ..] => "*/",
            [b'\'', ..] => "'",
            [b'(', ..] if close.is_some() => {
                after += 1;
                ""
            }
            [b')', ..] if close.is_some() => {
                if after == 0 {
                    ended = true;
                    break;
                }
                after -= 1;
                ""
            }
            [b'(', ..] if i >= open => {
                depth = Some(depth.map_or(1, |d| d + 1));
                ""
            }
            [b')', ..] if close.is_none() => {
                depth = depth.map(|d| d - 1);
                if depth == Some(0) {
                    close = Some(i);
                }
                ""
            }
            [b';', ..] if i < open => {
                first = i + 1;
                ""
            }
            [b';', ..] => {
                ended = true;
                break;
            }
            _ => "",
        };
        i += match skip {
            "" => 1,
            s => text
                .get(i + 1..)
                .and_then(|t| t.find(s))
                .map_or(b.len(), |n| n + 1 + s.len()),
        };
    }
    let recursive = Regex::new(r"(?i)\bWITH\s+RECURSIVE\b").expect("a valid pattern");
    let from = if text.get(first..open).is_some_and(|t| recursive.is_match(t)) {
        open
    } else {
        close.unwrap_or(i)
    };
    if at <= from {
        Some(false)
    } else {
        ended.then_some(at <= i)
    }
}

const SQL_IDENT: &str = r#"(?:"[^"]+"|`[^`]+`|[A-Za-z_]\w*)"#;

pub fn sql_local(text: &str, at: usize, word: &str) -> Vec<usize> {
    let (code, bodies) = sql_code(text);
    let line1 = |i: usize| text.bytes().take(i).filter(|&c| c == b'\n').count() + 1;
    match sql_aliases(&code, at, word) {
        None => Vec::new(),
        Some(found) if !found.is_empty() => found.into_iter().map(line1).collect(),
        Some(_) => sql_parameter(&code, &bodies, at, word)
            .map(line1)
            .into_iter()
            .collect(),
    }
}

pub(super) fn sql_string_end(b: &[u8], open: usize) -> Option<usize> {
    let escapes = open > 0
        && b[open - 1].eq_ignore_ascii_case(&b'e')
        && (open < 2 || !(b[open - 2].is_ascii_alphanumeric() || b[open - 2] == b'_'));
    let mut i = open + 1;
    while let Some(&c) = b.get(i) {
        match c {
            b'\\' if escapes => i += 2,
            b'\'' if b.get(i + 1) == Some(&b'\'') => i += 2,
            b'\'' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

fn sql_code(text: &str) -> (String, Vec<std::ops::Range<usize>>) {
    let tag = Regex::new(r"^\$(?:[A-Za-z_]\w*)?\$").expect("a valid pattern");
    let (b, mut out) = (text.as_bytes(), text.as_bytes().to_vec());
    let (mut bodies, mut open, mut i) = (Vec::new(), None::<(usize, &str)>, 0);
    while i < b.len() {
        let rest = text.get(i..).unwrap_or_default();
        let skip = match b[i] {
            b'-' if rest.starts_with("--") => rest.find('\n'),
            b'/' if rest.starts_with("/*") => {
                rest.get(2..).and_then(|r| r.find("*/")).map(|n| n + 4)
            }
            b'\'' => sql_string_end(b, i).map(|end| end - i),
            b'"' => {
                i += rest
                    .get(1..)
                    .and_then(|r| r.find('"'))
                    .map_or(b.len(), |n| n + 2);
                continue;
            }
            b'$' if i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_') => {
                let Some(d) = tag.find(rest).map(|m| m.as_str()) else {
                    i += 1;
                    continue;
                };
                match open {
                    None => open = Some((i + d.len(), d)),
                    Some((from, t)) if t == d => {
                        bodies.push(from..i);
                        open = None;
                    }
                    Some(_) => {}
                }
                out[i..i + d.len()].fill(b';');
                i += d.len();
                continue;
            }
            _ => {
                i += 1;
                continue;
            }
        };
        let end = skip.map_or(b.len(), |n| (i + n).min(b.len()));
        out[i..end]
            .iter_mut()
            .filter(|c| **c != b'\n')
            .for_each(|c| *c = b' ');
        i = end;
    }
    (String::from_utf8_lossy(&out).into_owned(), bodies)
}

fn sql_parameter(
    code: &str,
    bodies: &[std::ops::Range<usize>],
    at: usize,
    word: &str,
) -> Option<usize> {
    let body = bodies.iter().find(|b| b.contains(&at))?;
    let head_end = code.get(..body.start)?.trim_end_matches(';').len();
    let head_start = code.get(..head_end)?.rfind(';').map_or(0, |n| n + 1);
    let head = code.get(head_start..head_end)?;
    let create = Regex::new(&format!(
        r"(?i)\bCREATE\s+(?:OR\s+REPLACE\s+)?(?:FUNCTION|PROCEDURE)\s+{SQL_IDENT}(?:\s*\.\s*{SQL_IDENT})*\s*\("
    ))
    .expect("a valid pattern");
    let param = Regex::new(&format!(
        r"^\s*(?:(?i:IN|OUT|INOUT|VARIADIC)\s+)?({SQL_IDENT})\s+(\S+)"
    ))
    .expect("a valid pattern");
    let (mut from, mut depth) = (create.find(head)?.end(), 0_usize);
    for (i, c) in head.bytes().enumerate().skip(from) {
        if c == b'(' {
            depth += 1;
        } else if c == b')' && depth > 0 {
            depth -= 1;
        } else if (c == b',' || c == b')') && depth == 0 {
            let named = param.captures(head.get(from..i)?).filter(|p| {
                let (name, ty) = (&p[1], p[2].to_ascii_lowercase());
                !["in", "out", "inout", "variadic"].contains(&name.to_ascii_lowercase().as_str())
                    && !["precision", "varying", "with", "without"].contains(&ty.as_str())
                    && name.trim_matches(['"', '`']).eq_ignore_ascii_case(word)
            });
            if let Some(n) = named.and_then(|p| p.get(1)) {
                return Some(head_start + from + n.start());
            }
            if c == b')' {
                return None;
            }
            from = i + 1;
        }
    }
    None
}

const SQL_NOT_ALIAS: &[&str] = &[
    "and",
    "or",
    "not",
    "is",
    "in",
    "on",
    "using",
    "where",
    "join",
    "inner",
    "left",
    "right",
    "full",
    "outer",
    "cross",
    "natural",
    "lateral",
    "group",
    "order",
    "having",
    "limit",
    "offset",
    "fetch",
    "for",
    "window",
    "union",
    "except",
    "intersect",
    "minus",
    "returning",
    "set",
    "values",
    "select",
    "from",
    "with",
    "as",
    "into",
    "tablesample",
    "when",
    "then",
    "else",
    "end",
    "only",
    "straight_join",
    "use",
    "force",
    "ignore",
    "partition",
    "qualify",
    "connect",
    "start",
    "pivot",
    "unpivot",
    "go",
    "apply",
    "between",
    "like",
];

type SqlFromItem = (std::ops::Range<usize>, Option<std::ops::Range<usize>>);

fn sql_aliases(code: &str, at: usize, word: &str) -> Option<Vec<usize>> {
    let s = code.get(..at)?.rfind(';').map_or(0, |n| n + 1);
    let e = code.get(at..)?.find(';').map_or(code.len(), |n| at + n);
    let stmt = code.get(s..e)?;
    let mut found = Vec::new();
    for (table, alias) in sql_from_items(stmt) {
        let on = |r: &std::ops::Range<usize>| r.contains(&(at - s));
        if on(&table) || alias.as_ref().is_some_and(on) {
            return None;
        }
        let named = |r: &std::ops::Range<usize>| {
            stmt.get(r.clone())
                .is_some_and(|t| t.trim_matches(['"', '`']).eq_ignore_ascii_case(word))
        };
        found.extend(
            alias
                .filter(named)
                .and_then(|a| sql_scope_seen(stmt, a.start, at - s).map(|g| (g, s + a.start))),
        );
    }
    let inner = found.iter().map(|&(g, _)| g).max();
    Some(
        found
            .into_iter()
            .filter(|&(g, _)| Some(g) == inner)
            .map(|(_, a)| a)
            .collect(),
    )
}

fn sql_scope_seen(stmt: &str, alias: usize, at: usize) -> Option<usize> {
    let query = Regex::new(r"(?i)^\s*(?:SELECT|WITH|VALUES)\b").expect("a valid pattern");
    let set = Regex::new(r"(?i)\b(?:UNION|EXCEPT|INTERSECT)\b").expect("a valid pattern");
    let b = stmt.as_bytes();
    let (mut depth, mut open) = (0_usize, None);
    for i in (0..alias).rev() {
        match b[i] {
            b')' => depth += 1,
            b'(' if depth > 0 => depth -= 1,
            b'(' if stmt.get(i + 1..).is_some_and(|r| query.is_match(r)) => {
                open = Some(i);
                break;
            }
            _ => {}
        }
    }
    let from = open.map_or(0, |o| o + 1);
    let mut close = b.len();
    depth = 0;
    for (i, &c) in b.iter().enumerate().skip(from) {
        match c {
            b'(' => depth += 1,
            b')' if depth == 0 => {
                close = i;
                break;
            }
            b')' => depth -= 1,
            _ => {}
        }
    }
    if !(from..close).contains(&at) {
        return None;
    }
    let region = stmt.get(from..close)?;
    let level = |i: usize| {
        region.bytes().take(i).fold(0_isize, |d, c| match c {
            b'(' => d + 1,
            b')' => d - 1,
            _ => d,
        })
    };
    let (lo, hi) = (alias.min(at) - from, alias.max(at) - from);
    let split = set
        .find_iter(region)
        .any(|m| (lo..hi).contains(&m.start()) && level(m.start()) == 0);
    (!split).then_some(from)
}

fn sql_from_items(stmt: &str) -> Vec<SqlFromItem> {
    let start = Regex::new(r"(?i)\b(?:FROM|JOIN)\s").expect("a valid pattern");
    let name = Regex::new(&format!(
        r"^\s*(?:(?i:ONLY)\s+)?({SQL_IDENT}(?:\s*\.\s*{SQL_IDENT})*)"
    ))
    .expect("a valid pattern");
    let alias = Regex::new(&format!(r"^(?:\s+(?i:AS))?\s+({SQL_IDENT})")).expect("a valid pattern");
    let keyword = |r: &std::ops::Range<usize>| {
        stmt.get(r.clone())
            .is_some_and(|t| SQL_NOT_ALIAS.contains(&t.to_ascii_lowercase().as_str()))
    };
    let mut items = Vec::new();
    for m in start.find_iter(stmt) {
        let mut i = m.end();
        while let Some(n) = stmt
            .get(i..)
            .and_then(|r| name.captures(r))
            .and_then(|c| c.get(1))
        {
            let table = i + n.start()..i + n.end();
            if keyword(&table) {
                break;
            }
            i = table.end;
            let named = stmt
                .get(i..)
                .and_then(|r| alias.captures(r))
                .and_then(|c| c.get(1));
            let aliased = named.map(|a| i + a.start()..i + a.end()).filter(|a| {
                !keyword(a)
                    && !stmt
                        .get(a.end..)
                        .is_some_and(|r| r.trim_start().starts_with('.'))
            });
            if let Some(a) = &aliased {
                i = a.end;
            }
            items.push((table, aliased));
            match stmt.get(i..).and_then(|r| r.trim_start().strip_prefix(',')) {
                Some(r) => i = stmt.len() - r.len(),
                None => break,
            }
        }
    }
    items
}

pub fn sql_same_schema(lead: &str, decl: &str) -> Option<bool> {
    let unquote = |s: &str| s.trim_matches(['"', '`']).to_ascii_lowercase();
    let used = Regex::new(&format!(r"({SQL_NAME_PART})\s*\.\s*$")).expect("a valid pattern");
    let declared =
        Regex::new(&format!(r"{}({SQL_NAME_PART})\s*\.", sql_create!())).expect("a valid pattern");
    let used = unquote(&used.captures(lead)?[1]);
    Some(
        declared
            .captures(decl)
            .is_some_and(|c| unquote(&c[1]) == used),
    )
}

pub fn sql_renames(line: &str) -> bool {
    Regex::new(r"(?i)^\s*ALTER\s.*\sRENAME\s+TO\s")
        .expect("a valid pattern")
        .is_match(line)
}

fn sql_migration(path: &Path) -> Option<(&Path, &str, bool)> {
    let down = match path.file_name()?.to_str()? {
        "up.sql" => false,
        "down.sql" => true,
        _ => return None,
    };
    let folder = path.parent()?;
    Some((folder.parent()?, folder.file_name()?.to_str()?, down))
}

fn sql_runs_before(a: &str, b: &str) -> Option<bool> {
    let version = |f: &str| -> Option<String> {
        let v: String = f.split('_').next()?.chars().filter(|&c| c != '-').collect();
        (!v.is_empty() && v.bytes().all(|c| c.is_ascii_digit())).then_some(v)
    };
    let (x, y) = (version(a)?, version(b)?);
    if x.starts_with(&y) || y.starts_with(&x) {
        return None;
    }
    let numeric = |v: &str| {
        let t = v.trim_start_matches('0');
        (t.len(), t.to_owned())
    };
    let before = x < y;
    (before == (numeric(&x) < numeric(&y))).then_some(before)
}

pub fn sql_in_effect(here: &Path, line1: usize, hits: &[Hit]) -> Option<Vec<usize>> {
    let (dir, cursor, down) = sql_migration(here)?;
    let object = Regex::new(
        r"(?i)\b(TABLE|VIEW|INDEX|FUNCTION|PROCEDURE|TRIGGER|TYPE|SCHEMA|SEQUENCE|DOMAIN|EXTENSION|DATABASE|ROLE|USER|ENUM)\b",
    )
    .expect("a valid pattern");
    let mut best: Vec<(String, usize, &str, bool)> = Vec::new();
    for (i, h) in hits.iter().enumerate() {
        let Some((_, folder, h_down)) = sql_migration(&h.path).filter(|m| m.0 == dir) else {
            continue;
        };
        let earlier = if folder == cursor {
            if h.path == here {
                h.line1 < line1
            } else {
                down && !h_down
            }
        } else {
            !h_down && sql_runs_before(folder, cursor)?
        };
        if !earlier {
            continue;
        }
        let kind = object.find(&h.text)?.as_str().to_ascii_lowercase();
        match best.iter_mut().find(|b| b.0 == kind) {
            None => best.push((kind, i, folder, h_down)),
            Some(b) => {
                let later = if b.2 == folder {
                    (h_down, h.line1) > (b.3, hits[b.1].line1)
                } else {
                    sql_runs_before(b.2, folder)?
                };
                if later {
                    *b = (kind, i, folder, h_down);
                }
            }
        }
    }
    Some(best.into_iter().map(|b| b.1).collect())
}

pub fn sql_alias_table(text: &str, at: usize, qualifier: &str) -> Option<String> {
    let (code, _) = sql_code(text);
    let s = code.get(..at)?.rfind(';').map_or(0, |n| n + 1);
    let e = code.get(at..)?.find(';').map_or(code.len(), |n| at + n);
    let stmt = code.get(s..e)?;
    let unquote = |t: &str| t.trim().trim_matches(['"', '`']).to_ascii_lowercase();
    let q = unquote(qualifier);
    let items = sql_from_items(stmt);
    let table = |r: &std::ops::Range<usize>| {
        let parts: Vec<String> = stmt.get(r.clone())?.split('.').map(unquote).collect();
        Some(parts.join("."))
    };
    let last = |t: &str| t.rsplit('.').next().unwrap_or(t).to_owned();
    let aliased = |a: &Option<std::ops::Range<usize>>| {
        a.as_ref().and_then(|a| stmt.get(a.clone())).map(unquote)
    };
    let mut tables: Vec<String> = items
        .iter()
        .filter(|(_, a)| aliased(a).as_ref() == Some(&q))
        .filter_map(|(t, _)| table(t))
        .collect();
    if tables.is_empty() {
        tables = items
            .iter()
            .filter(|(_, a)| a.is_none())
            .filter_map(|(t, _)| table(t))
            .filter(|t| last(t) == q)
            .collect();
    }
    tables.sort();
    tables.dedup();
    let cte = Regex::new(&sql_cte(&last(tables.first()?))).expect("an escaped name keeps it valid");
    match tables.as_slice() {
        [t] if !stmt.lines().any(|l| cte.is_match(l)) => Some(t.clone()),
        _ => None,
    }
}

pub fn sql_column_pattern(table: &str, column: &str) -> String {
    let name = |n: &str| {
        let n = regex::escape(n);
        format!(r#"(?:"{n}"|{n}\b)"#)
    };
    let table = table.split('.').map(name).collect::<Vec<_>>().join(r"\s*\.\s*");
    let column = name(column);
    let alter = format!(r"(?i)^\s*ALTER\s+TABLE\s+(?:IF\s+EXISTS\s+)?(?:ONLY\s+)?{table}");
    format!(
        r"(?i)^\s*CREATE\s+(?:(?:TEMP|TEMPORARY|UNLOGGED)\s+)*TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?{table}\s*(?:\(|$)|{alter}.*\sADD\s+(?:COLUMN\s+)?(?:IF\s+NOT\s+EXISTS\s+)?{column}|{alter}\s+RENAME\s+(?:COLUMN\s+)?\S+\s+TO\s+{column}"
    )
}

pub fn sql_table_column(text: &str, create: usize, column: &str) -> Option<usize> {
    let (code, _) = sql_code(text);
    let from: usize = code
        .split_inclusive('\n')
        .take(create - 1)
        .map(str::len)
        .sum();
    let rest = code.get(from..)?;
    let open = from + rest.find('(')?;
    if code.get(from..open)?.contains(';') {
        return None;
    }
    let item = Regex::new(&format!(
        r#"(?i)^\s*(?:"{c}"|{c})\s+[A-Za-z]"#,
        c = regex::escape(column)
    ))
    .expect("an escaped name keeps it valid");
    let (b, mut depth, mut start) = (code.as_bytes(), 0_usize, open + 1);
    for (i, &c) in b.iter().enumerate().skip(open) {
        match c {
            b'(' => depth += 1,
            b')' | b',' if depth == 1 => {
                if item.is_match(code.get(start..i)?) {
                    let lead = code.get(start..i)?.len() - code.get(start..i)?.trim_start().len();
                    return Some(code.get(..start + lead)?.matches('\n').count() + 1);
                }
                if c == b')' {
                    return None;
                }
                start = i + 1;
            }
            b')' => depth = depth.saturating_sub(1),
            b';' => return None,
            _ => {}
        }
    }
    None
}
