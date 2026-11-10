use regex::Regex;

use super::{Kind, uncommented};

pub struct GenericHeader {
    pub params: Vec<String>,
    pub rest: String,
}

pub fn generic_header(
    kind: Kind,
    text: &str,
    decl_line1: usize,
    name: &str,
) -> Option<GenericHeader> {
    let k = decl_line1.checked_sub(1)?;
    let mut header = String::new();
    for l in text.lines().skip(k).take(20) {
        let code = uncommented(kind, l);
        let (head, done) = code
            .split_once('{')
            .map_or((code.as_str(), false), |(h, _)| (h, true));
        header.push_str(head);
        header.push(' ');
        if done {
            break;
        }
    }
    let mut flat = String::new();
    let mut parens = 0i32;
    for c in header.chars() {
        match c {
            '(' => parens += 1,
            ')' => parens -= 1,
            _ if parens == 0 => flat.push(c),
            _ => {}
        }
    }
    let head = Regex::new(&format!(
        r"\b(?:class|interface|record|struct|object)\s+{}\b\s*",
        regex::escape(name)
    ))
    .ok()?;
    let after: String = flat.get(head.find(&flat)?.end()..)?.to_owned();
    let Some(inner) = after.strip_prefix('<') else {
        return Some(GenericHeader {
            params: Vec::new(),
            rest: after,
        });
    };
    let (list, rest) = angle_group(inner)?;
    let params = top_commas(&list)
        .iter()
        .map(|p| {
            p.split_whitespace()
                .find(|w| !matches!(*w, "in" | "out" | "reified"))
                .unwrap_or("")
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        })
        .collect();
    Some(GenericHeader { params, rest })
}

pub fn base_arguments(rest: &str, base: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = rest.chars().collect();
    let name: Vec<char> = base.chars().collect();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut depth = 0i32;
    let mut found = None;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '<' => depth += 1,
            '>' if i == 0 || !matches!(chars[i - 1], '-' | '=') => depth -= 1,
            _ if depth == 0
                && chars.get(i..i + name.len()) == Some(&name[..])
                && (i == 0 || !word(chars[i - 1]))
                && !chars.get(i + name.len()).copied().is_some_and(word) =>
            {
                let after: String = chars.get(i + name.len()..)?.iter().collect();
                let Some(inner) = after.trim_start().strip_prefix('<') else {
                    return None;
                };
                if found.is_some() {
                    return None;
                }
                found = Some(top_commas(&angle_group(inner)?.0));
                i += name.len();
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    found
}

fn angle_group(inner: &str) -> Option<(String, String)> {
    let mut depth = 1i32;
    let mut prev = ' ';
    for (i, c) in inner.char_indices() {
        match c {
            '<' => depth += 1,
            '>' if !matches!(prev, '-' | '=') => {
                depth -= 1;
                if depth == 0 {
                    return Some((inner.get(..i)?.to_owned(), inner.get(i + 1..)?.to_owned()));
                }
            }
            _ => {}
        }
        prev = c;
    }
    None
}

fn top_commas(list: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut depth = 0i32;
    let mut prev = ' ';
    for c in list.chars() {
        match c {
            '<' | '[' => depth += 1,
            '>' if !matches!(prev, '-' | '=') => depth -= 1,
            ']' => depth -= 1,
            ',' if depth == 0 => {
                out.push(String::new());
                prev = c;
                continue;
            }
            _ => {}
        }
        if let Some(last) = out.last_mut() {
            last.push(c);
        }
        prev = c;
    }
    out.iter().map(|s| s.trim().to_owned()).collect()
}
