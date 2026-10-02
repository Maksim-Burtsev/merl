use super::*;
use regex::Regex;
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum RefAt {
    Value(String),
    Refused,
}

pub fn is_ref_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e == "yaml" || e == "yml" || e == "json")
}

fn is_json(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "json")
}

pub fn ref_at(path: &Path, lines: &[&str], line: usize, col: usize) -> Option<RefAt> {
    static REF: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?:^|[\s{,])(?:"\$ref"|'\$ref'|\$ref)\s*:[ \t]*"#).unwrap()
    });
    let json = is_json(path);
    let l = *lines.get(line)?;
    let at_ref = REF.find_iter(l).find_map(|m| {
        let (span, value) = value_at(l, m.end())?;
        span.contains(&col).then_some((m.start(), value))
    });
    let (start, value) = match at_ref {
        Some(found) => found,
        None => {
            let t = l.trim_start();
            let body = t.strip_prefix("- ").unwrap_or(t);
            let (_, from) = key_of(body)?;
            let (span, value) = value_at(l, l.len() - body.len() + from)?;
            if !span.contains(&col) || !in_mapping(lines, line, json) {
                return None;
            }
            (0, value)
        }
    };
    let comment = if json { "//" } else { "#" };
    let refused = l.trim_start().starts_with(comment)
        || (!json && comment_start(l).is_some_and(|c| c < start))
        || (!json && in_block_scalar(lines, line));
    Some(match refused {
        true => RefAt::Refused,
        false => RefAt::Value(value),
    })
}

fn in_mapping(lines: &[&str], line: usize, json: bool) -> bool {
    let key = |i: usize| {
        let t = lines[i].trim_start();
        key_of(t.strip_prefix("- ").unwrap_or(t)).map(|(k, _)| k)
    };
    let Some(mapping) = parent(lines, line, json) else {
        return false;
    };
    let Some(discriminator) = parent(lines, mapping, json) else {
        return false;
    };
    key(mapping).as_deref() == Some("mapping")
        && key(discriminator).as_deref() == Some("discriminator")
}

fn parent(lines: &[&str], line: usize, json: bool) -> Option<usize> {
    let depth = syntax::indent(lines[line]);
    (0..line)
        .rev()
        .find(|&i| !skipped(lines[i].trim(), json) && syntax::indent(lines[i]) < depth)
}

fn in_block_scalar(lines: &[&str], line: usize) -> bool {
    static HEADER: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"(?:^-|:)\s+[|>][-+0-9]*\s*(?:#.*)?$").unwrap());
    let mut at = line;
    while let Some(up) = parent(lines, at, false) {
        if HEADER.is_match(lines[up].trim()) {
            return true;
        }
        at = up;
    }
    false
}

fn comment_start(l: &str) -> Option<usize> {
    let mut quote = None;
    let mut prev = b' ';
    for (i, c) in l.bytes().enumerate() {
        match (quote, c) {
            (None, b'"' | b'\'') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, b'#') if prev.is_ascii_whitespace() => return Some(i),
            _ => {}
        }
        prev = c;
    }
    None
}

fn value_at(l: &str, from: usize) -> Option<(std::ops::Range<usize>, String)> {
    let rest = &l[from..];
    match rest.as_bytes().first()? {
        q @ (b'"' | b'\'') => {
            let close = rest[1..].find(*q as char)? + 1;
            Some((from..from + close + 1, rest[1..close].to_owned()))
        }
        _ => {
            let end = [" #", ",", "}"]
                .iter()
                .filter_map(|s| rest.find(s))
                .min()
                .unwrap_or(rest.len());
            let value = rest[..end].trim_end();
            (!value.is_empty()).then(|| (from..from + value.len(), value.to_owned()))
        }
    }
}

fn key_of(body: &str) -> Option<(String, usize)> {
    let (key, after) = match body.as_bytes().first()? {
        q @ (b'"' | b'\'') => {
            let close = body[1..].find(*q as char)? + 1;
            (body[1..close].to_owned(), close + 1)
        }
        _ => {
            let colon = body
                .match_indices(':')
                .map(|(i, _)| i)
                .find(|&i| body[i + 1..].is_empty() || body[i + 1..].starts_with([' ', '\t']))?;
            (body[..colon].trim_end().to_owned(), colon)
        }
    };
    let rest = body[after..].trim_start().strip_prefix(':')?;
    let value = rest.trim_start();
    Some((key, body.len() - value.len()))
}

fn skipped(t: &str, json: bool) -> bool {
    t.is_empty()
        || match json {
            true => t.starts_with("//"),
            false => t.starts_with('#') || t == "---" || t == "..." || t.starts_with('%'),
        }
}

fn closer(t: &str) -> bool {
    t.starts_with(['}', ']'])
}

pub fn pointer_line(path: &Path, text: &str, pointer: &str) -> Option<usize> {
    let json = is_json(path);
    let rows: Vec<(usize, usize, &str)> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| !skipped(l.trim(), json))
        .map(|(i, l)| (i, syntax::indent(l), l.trim()))
        .collect();
    let segments: Vec<String> = percent_decoded(pointer.strip_prefix('/')?)
        .split('/')
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect();
    let (mut lo, mut hi) = (0, rows.len());
    let mut array = false;
    let mut item: Option<usize> = None;
    if let Some(&(_, _, body @ ("{" | "["))) = rows.first() {
        (lo, array) = (1, body == "[");
    }
    let mut found = None;
    for seg in segments {
        let body = |r: usize| match Some(r) == item {
            true => rows[r].2[1..].trim_start(),
            false => rows[r].2,
        };
        let depth = |r: usize| rows[r].1 + rows[r].2.len() - body(r).len();
        let first = (lo..hi).find(|&r| !closer(rows[r].2))?;
        let c = depth(first);
        let mut children = (lo..hi).filter(|&r| depth(r) == c && !closer(body(r)));
        let items = array || (!json && (body(first).starts_with("- ") || body(first) == "-"));
        let hit = match items {
            true => children.nth(seg.parse().ok()?)?,
            false => children.find(|&r| key_of(body(r)).is_some_and(|(k, _)| k == seg))?,
        };
        let end = (hit + 1..hi)
            .find(|&r| {
                let d = rows[r].1;
                d < c || (d == c && (items || !rows[r].2.starts_with('-') || json))
            })
            .unwrap_or(hi);
        let value = match items {
            true if json => body(hit),
            true => body(hit).strip_prefix('-').unwrap_or("").trim_start(),
            false => {
                let (_, v) = key_of(body(hit))?;
                &body(hit)[v..]
            }
        };
        let value = match json {
            true => value,
            false => value.split_once(" #").map_or(value, |(v, _)| v).trim(),
        };
        let value = match !json && value.starts_with(['&', '!']) {
            true => value.split_once(' ').map_or("", |(_, v)| v).trim_start(),
            false => value,
        };
        (lo, hi, array, item) = match (items && !json && key_of(value).is_some(), value) {
            (true, _) => (hit, end, false, Some(hit)),
            (false, "" | "{") => (hit + 1, end, false, None),
            (false, "[") => (hit + 1, end, true, None),
            _ => (end, end, false, None),
        };
        found = Some(rows[hit].0 + 1);
    }
    found
}

pub fn keyword_line(text: &str, keyword: &str, value: &str) -> Option<usize> {
    let re = Regex::new(&format!(
        r#"(?:^|[\s{{,])(?:"{k}"|'{k}'|{k})\s*:[ \t]*"#,
        k = regex::escape(keyword)
    ))
    .unwrap();
    text.lines()
        .position(|l| {
            re.find_iter(l).any(|m| {
                value_at(l, m.end()).is_some_and(|(_, v)| v.trim_end_matches('#') == value)
            })
        })
        .map(|i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPENAPI: &str = "\
openapi: 3.1.0
paths:
  '/users/{id}':
    get:
      responses:
        \"200\":
          $ref: '#/components/responses/NotFound'
components:
  schemas:
    User:
      allOf:
      - $ref: '#/components/schemas/Base'
      - type: object
        properties:
          name: {type: string}
    Base:
      type: object
  responses:
    NotFound:
      description: |
        $ref: '#/components/schemas/User'
";

    #[test]
    fn a_pointer_walks_keys_by_indentation() {
        let p = Path::new("a.yaml");
        assert_eq!(
            pointer_line(p, OPENAPI, "/components/schemas/User"),
            Some(10)
        );
        assert_eq!(
            pointer_line(p, OPENAPI, "/components/responses/NotFound"),
            Some(19)
        );
        assert_eq!(
            pointer_line(p, OPENAPI, "/paths/~1users~1%7Bid%7D/get/responses/200"),
            Some(6)
        );
        assert_eq!(
            pointer_line(
                p,
                OPENAPI,
                "/components/schemas/User/allOf/1/properties/name"
            ),
            Some(15)
        );
        assert_eq!(
            pointer_line(p, OPENAPI, "/components/schemas/User/allOf/1"),
            Some(13)
        );
        assert_eq!(
            pointer_line(
                p,
                OPENAPI,
                "/components/schemas/User/allOf/1/properties/name/type"
            ),
            None
        );
        assert_eq!(pointer_line(p, OPENAPI, "/schemas/User"), None);
        assert_eq!(
            pointer_line(p, OPENAPI, "/components/responses/NotFound/description/x"),
            None
        );
    }

    #[test]
    fn a_pointer_walks_pretty_json() {
        let json = "{\n  \"$defs\": {\n    \"address\": {\n      \"type\": \"object\"\n    },\n    \"list\": [\n      { \"type\": \"string\" },\n      {\n        \"type\": \"number\"\n      }\n    ]\n  }\n}";
        let p = Path::new("a.json");
        assert_eq!(pointer_line(p, json, "/$defs/address"), Some(3));
        assert_eq!(pointer_line(p, json, "/$defs/address/type"), Some(4));
        assert_eq!(pointer_line(p, json, "/$defs/list/1/type"), Some(9));
        assert_eq!(pointer_line(p, json, "/$defs/list/0/type"), None);
        assert_eq!(pointer_line(p, "{\"a\": {\"b\": 1}}", "/a/b"), None);
    }

    #[test]
    fn the_reference_under_the_cursor() {
        let lines: Vec<&str> = OPENAPI.lines().collect();
        let p = Path::new("a.yaml");
        let value = RefAt::Value("#/components/responses/NotFound".into());
        assert_eq!(ref_at(p, &lines, 6, 20), Some(value));
        assert_eq!(ref_at(p, &lines, 6, 10), None);
        assert_eq!(ref_at(p, &lines, 20, 20), Some(RefAt::Refused));
        let commented = ["# $ref: '#/a'"];
        assert_eq!(ref_at(p, &commented, 0, 10), Some(RefAt::Refused));
        let json = ["    \"home\": { \"$ref\": \"#/$defs/address\" },"];
        let value = RefAt::Value("#/$defs/address".into());
        assert_eq!(ref_at(Path::new("a.json"), &json, 0, 30), Some(value));
        let mapping = [
            "discriminator:",
            "  mapping:",
            "    dog: Dog.yaml",
            "  dog: Dog.yaml",
        ];
        assert_eq!(
            ref_at(p, &mapping, 2, 10),
            Some(RefAt::Value("Dog.yaml".into()))
        );
        assert_eq!(ref_at(p, &mapping, 3, 8), None);
    }
}
