use std::ops::Range;
use std::path::Path;

use regex::Regex;

use super::{Binding, Kind, Value, indent, split_top};

pub fn reads_jsdoc(kind: Kind, path: &Path) -> bool {
    kind == Kind::TsJs
        && path
            .extension()
            .is_some_and(|e| matches!(e.to_str(), Some("js" | "jsx" | "mjs" | "cjs")))
}

pub(super) fn behind_doc(kind: Kind, line: &str) -> &str {
    let t = line.trim_start();
    let declares = |c: &str| ["const ", "let ", "var "].iter().any(|k| c.starts_with(k));
    match t.strip_prefix("/*").and_then(|c| c.split_once("*/")) {
        Some((_, code)) if kind == Kind::TsJs && declares(code.trim_start()) => code,
        _ => line,
    }
}

pub fn jsdoc_type(braced: &str) -> Option<String> {
    static ONE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*(?:\[\])?|Array<\s*[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*\s*>)$").expect("the regex is a literal")
    });
    let t = braced
        .trim()
        .trim_start_matches(['?', '!'])
        .trim_end_matches('=');
    let left: Vec<&str> = split_top(Kind::TsJs, t, b'|')
        .into_iter()
        .map(str::trim)
        .filter(|p| !matches!(*p, "null" | "undefined"))
        .collect();
    let [one] = left[..] else { return None };
    (ONE.is_match(one) && one != "any").then(|| one.to_owned())
}

fn braced(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start().strip_prefix('{')?;
    let mut depth = 1;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((s.get(..i)?, s.get(i + 1..)?));
                }
            }
            _ => {}
        }
    }
    None
}

struct Tag {
    name: String,
    braced_type: String,
    rest_of_line: String,
}

fn tags(doc: &[&str]) -> Vec<Tag> {
    let mut out = Vec::new();
    for l in doc {
        let l = l.trim().trim_start_matches("/**").trim_start_matches('*');
        let l = l.trim_end().trim_end_matches("*/");
        let Some(at) = l.trim_start().strip_prefix('@') else {
            continue;
        };
        let tag: String = at.chars().take_while(|c| c.is_alphanumeric()).collect();
        let Some((ty, rest)) = at.get(tag.len()..).and_then(braced) else {
            continue;
        };
        out.push(Tag {
            name: tag,
            braced_type: ty.to_owned(),
            rest_of_line: rest.trim().to_owned(),
        });
    }
    out
}

fn tag_name(rest: &str) -> Option<&str> {
    let rest = rest.strip_prefix('[').unwrap_or(rest);
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(rest.len());
    let (name, after) = rest.split_at_checked(end)?;
    (end > 0 && !after.starts_with('.')).then_some(name)
}

fn block_above(lines: &[&str], line0: usize) -> Option<Range<usize>> {
    let end = line0.checked_sub(1)?;
    if !lines[end].trim_end().ends_with("*/") {
        return None;
    }
    let start = (0..=end)
        .rev()
        .find(|&i| lines[i].trim_start().starts_with("/*"))?;
    (lines[start].trim_start().starts_with("/**")
        && !lines[start..end].iter().any(|l| l.contains("*/")))
    .then_some(start..end + 1)
}

pub fn jsdoc_binding(text: &str, line1: usize, name: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let k = line1.checked_sub(1)?;
    let own = lines.get(k)?;
    let declares = Regex::new(&format!(
        r"^\s*(?:/\*\*.*?\*/\s*)?(?:export\s+)?(?:const|let|var)\s+{}(?:[^\w$]|$)",
        regex::escape(name)
    ))
    .ok()?;
    let doc_on_the_line = own
        .trim_start()
        .starts_with("/**")
        .then(|| own.split_once("*/").map(|(doc, _)| doc))
        .flatten();
    let doc: Vec<&str> = match (doc_on_the_line, block_above(&lines, k)) {
        (Some(doc), _) => vec![doc],
        (None, Some(block)) => lines[block].to_vec(),
        (None, None) => return None,
    };
    let tags = tags(&doc);
    if declares.is_match(own) {
        let tag = tags.iter().find(|t| t.name == "type")?;
        return jsdoc_type(&tag.braced_type);
    }
    let tag = tags
        .iter()
        .find(|t| t.name == "param" && tag_name(&t.rest_of_line) == Some(name))?;
    jsdoc_type(&tag.braced_type)
}

pub fn jsdoc_returns(text: &str, decl_line1: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let block = block_above(&lines, decl_line1.checked_sub(1)?)?;
    let tags = tags(&lines[block]);
    let tag = tags
        .iter()
        .find(|t| t.name == "returns" || t.name == "return")?;
    jsdoc_type(&tag.braced_type)
}

pub fn js_assigned_calls(text: &str, decl_line1: usize, name: &str) -> Option<Vec<String>> {
    let n = regex::escape(name);
    let bare = Regex::new(&format!(r"^\s*(?:let|var)\s+{n}\s*;?\s*$")).ok()?;
    let assigned = Regex::new(&format!(r"^\s*{n}\s*=([^=].*)$")).ok()?;
    let call =
        Regex::new(r"^\s*(?:await\s+)?([A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*)\s*\(").ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let decl = lines.get(decl_line1.checked_sub(1)?)?;
    if !bare.is_match(decl) {
        return None;
    }
    let depth = indent(decl);
    let mut out = Vec::new();
    for l in lines.iter().skip(decl_line1) {
        if !l.trim().is_empty() && indent(l) < depth {
            break;
        }
        let Some(c) = assigned.captures(l) else {
            continue;
        };
        let expr = c[1].trim_start();
        match call.captures(expr) {
            Some(callee) => out.push(callee[1].to_owned()),
            None if expr.starts_with('{') => {}
            None => return None,
        }
    }
    Some(out)
}

pub fn jsdoc_returned_key(text: &str, decl_line1: usize, word: &str) -> Option<usize> {
    let lines: Vec<&str> = text.lines().collect();
    let block = block_above(&lines, decl_line1.checked_sub(1)?)?;
    let key = Regex::new(&format!(r"[{{,]\s*{}\s*\??\s*:", regex::escape(word))).ok()?;
    block
        .into_iter()
        .find(|&i| jsdoc_returned_literal(lines[i]).is_some_and(|t| key.is_match(&t)))
        .map(|i| i + 1)
}

pub fn jsdoc_returned_literal(line: &str) -> Option<String> {
    tags(&[line])
        .into_iter()
        .find(|t| {
            matches!(t.name.as_str(), "returns" | "return")
                && t.braced_type.trim_start().starts_with('{')
        })
        .map(|t| t.braced_type)
}

fn typedef(line: &str) -> Option<(&str, &str)> {
    let t = line.trim_start();
    let t = t.strip_prefix("/**").or_else(|| t.strip_prefix('*'))?;
    let (ty, rest) = braced(t.trim_start().strip_prefix("@typedef")?)?;
    let name = rest
        .trim_start()
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .next()
        .filter(|n| !n.is_empty())?;
    Some((ty.trim(), name))
}

pub struct JsdocTypedef {
    pub line1: usize,
    pub written_type: String,
}
pub fn jsdoc_typedefs(text: &str, name: &str) -> Vec<JsdocTypedef> {
    text.lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let (ty, n) = typedef(l)?;
            (n == name).then(|| JsdocTypedef {
                line1: i + 1,
                written_type: ty.to_owned(),
            })
        })
        .collect()
}

pub fn jsdoc_object(written: &str) -> bool {
    matches!(written, "Object" | "object")
}

pub fn jsdoc_owner_name<'a>(lines: &[&'a str], line0: usize) -> Option<&'a str> {
    let i = jsdoc_owner(lines, line0)?;
    typedef(lines[i]).map(|(_, name)| name)
}

pub fn jsdoc_owner(lines: &[&str], line0: usize) -> Option<usize> {
    if !lines[line0].contains("@prop") {
        return None;
    }
    for i in (0..=line0).rev() {
        if let Some((ty, _)) = typedef(lines[i]) {
            return jsdoc_object(ty).then_some(i);
        }
        let t = lines[i].trim_start();
        if !t.starts_with('*') || (i < line0 && t.contains("*/")) {
            return None;
        }
    }
    None
}

pub fn jsdoc_properties(lines: &[&str], line0: usize, name: &str) -> Option<Vec<Binding>> {
    typedef(lines[line0]).filter(|(ty, _)| jsdoc_object(ty))?;
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(line0 + 1) {
        if typedef(l).is_some() || !l.trim_start().starts_with('*') {
            break;
        }
        if let [tag] = &tags(&[l])[..]
            && (tag.name == "property" || tag.name == "prop")
            && tag_name(&tag.rest_of_line) == Some(name)
        {
            let value = jsdoc_type(&tag.braced_type).map_or(Value::Unknown, Value::Type);
            out.push(Binding {
                line1: i + 1,
                value,
            });
        }
        if l.contains("*/") {
            break;
        }
    }
    Some(out)
}
pub fn jsdoc_braced_at(line: &str, word: Range<usize>, in_comment: bool) -> bool {
    let t = line.trim_start();
    let (Some(before), Some(after)) = (line.get(..word.start), line.get(word.end..)) else {
        return false;
    };
    (t.starts_with("/**") || (in_comment && t.starts_with('*') && !t.starts_with("*/")))
        && !before.contains("*/")
        && before.rfind('{') > before.rfind('}')
        && after
            .find('}')
            .is_some_and(|c| after.find('{').is_none_or(|o| c < o))
}
pub fn jsdoc_typedef_line(line: &str) -> bool {
    typedef(line).is_some()
}

pub fn eslint_visited_node(text: &str, line1: usize, name: &str) -> Option<String> {
    static VISITOR: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"^(\s*)(?:([A-Z][A-Za-z]*)|"([A-Z][A-Za-z]*)(?::exit)?"|'([A-Z][A-Za-z]*)(?::exit)?')\s*\(\s*([A-Za-z_$][\w$]*)\s*\)\s*\{\s*$"#)
            .expect("the regex is a literal")
    });
    static CREATE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*create\s*(?::\s*function\s*)?\(\s*[A-Za-z_$][\w$]*\s*\)\s*\{\s*$")
            .expect("the regex is a literal")
    });
    let lines: Vec<&str> = text.lines().collect();
    let k = line1.checked_sub(1)?;
    let c = VISITOR.captures(lines.get(k)?)?;
    if &c[5] != name {
        return None;
    }
    let indent = |l: &str| l.len() - l.trim_start().len();
    let outer = |below: usize, than: usize| {
        (0..below)
            .rev()
            .find(|&i| !lines[i].trim().is_empty() && indent(lines[i]) < than)
    };
    let opener = outer(k, c[1].len())?;
    let create = outer(opener, indent(lines[opener]))?;
    (lines[opener].trim() == "return {" && CREATE.is_match(lines[create]))
        .then(|| (2..=4).find_map(|i| c.get(i)))
        .flatten()
        .map(|m| m.as_str().to_owned())
}
