//! The types a JavaScript file writes in JSDoc (#347): `@type`, `@param`, `@returns`, and the
//! fields of a `@typedef {Object}`. They are read in `.js`, `.jsx`, `.mjs` and `.cjs` files only
//! ([`reads_jsdoc`], which every reader of them asks): a `.ts` file writes annotations, and
//! TypeScript ignores JSDoc types there.

use std::ops::Range;
use std::path::Path;

use regex::Regex;

use super::*;

/// Whether the file `path` of `kind` is JavaScript, whose types JSDoc writes: the one test of
/// every rule of this module.
pub fn reads_jsdoc(kind: Kind, path: &Path) -> bool {
    kind == Kind::TsJs
        && path
            .extension()
            .is_some_and(|e| matches!(e.to_str(), Some("js" | "jsx" | "mjs" | "cjs")))
}

/// The `const`, `let` or `var` of a TypeScript or JavaScript `line` behind the block comment it
/// opens with and closes on, `/** @type {T} */ let x;`: a declaration all the same. Any other
/// line as it is, a comment.
pub(super) fn behind_doc(kind: Kind, line: &str) -> &str {
    let t = line.trim_start();
    let declares = |c: &str| ["const ", "let ", "var "].iter().any(|k| c.starts_with(k));
    match t.strip_prefix("/*").and_then(|c| c.split_once("*/")) {
        Some((_, code)) if kind == Kind::TsJs && declares(code.trim_start()) => code,
        _ => line,
    }
}

/// The type inside a JSDoc tag's braces when it is one plain name, as an annotation is read:
/// `Name`, `ns.Name`, `Name[]` or `Array<Name>`, behind `?` or `!`, or in a union with `null` or
/// `undefined`. A union of two types, another generic, an inline object or function type, `*`
/// and `any` type nothing.
pub fn jsdoc_type(braced: &str) -> Option<String> {
    static ONE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^(?:[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*(?:\[\])?|Array<\s*[A-Za-z_$][\w$]*(?:\.[A-Za-z_$][\w$]*)*\s*>)$").unwrap()
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

/// `{…}` at the start of `s`, braces inside it balanced: what it holds, and what follows it.
fn braced(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start().strip_prefix('{')?;
    let mut depth = 1;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[..i], &s[i + 1..]));
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
        let Some((ty, rest)) = braced(&at[tag.len()..]) else {
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

/// The name a `@param` or `@property` names, `name` or `[name=default]`; none for a part of one,
/// `options.cwd`, which types a destructured parameter or a nested field.
fn tag_name(rest: &str) -> Option<&str> {
    let rest = rest.strip_prefix('[').unwrap_or(rest);
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .unwrap_or(rest.len());
    (end > 0 && !rest[end..].starts_with('.')).then(|| &rest[..end])
}

/// The lines of the JSDoc block `/** … */` that closes right above `line0`: none when a blank
/// line, code or a comment of another kind stands between them.
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

/// The type JSDoc writes for `name` bound on `line1` of `text`: the `@type {T}` of a `const`,
/// `let` or `var` declaring it, on its line or right above it; else the `@param {T} name` of the
/// function whose header starts on the line.
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

/// The `@returns {T}` or `@return {T}` of the JSDoc block right above the function or method
/// declared on `decl_line1` of `text`.
pub fn jsdoc_returns(text: &str, decl_line1: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let block = block_above(&lines, decl_line1.checked_sub(1)?)?;
    let tags = tags(&lines[block]);
    let tag = tags
        .iter()
        .find(|t| t.name == "returns" || t.name == "return")?;
    jsdoc_type(&tag.braced_type)
}

/// The `@typedef {T} Name` a JSDoc line writes: `T` and `Name`.
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

/// Each `@typedef {T} name` of `text`, by its 1-based line, with `T` as written.
pub fn jsdoc_typedefs(text: &str, name: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let (ty, n) = typedef(l)?;
            (n == name).then(|| (i + 1, ty.to_owned()))
        })
        .collect()
}

/// Whether `written`, a `@typedef`'s type, declares fields of its own: `Object` or `object`.
pub fn jsdoc_object(written: &str) -> bool {
    matches!(written, "Object" | "object")
}

/// The name of the `@typedef {Object}` whose block holds the `@property` on `line0`.
pub fn jsdoc_owner_name<'a>(lines: &[&'a str], line0: usize) -> Option<&'a str> {
    let i = jsdoc_owner(lines, line0)?;
    typedef(lines[i]).map(|(_, name)| name)
}

/// The 0-based line of the `@typedef {Object}` whose block holds the `@property` on `line0`.
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

/// The fields `name` the `@typedef {Object}` on `line0` declares: each `@property {T} name` or
/// `@property {T} [name]` of its block, with `T` when it is one plain name. `None` when the line
/// writes no such typedef.
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
            out.push(Binding { line: i + 1, value });
        }
        if l.contains("*/") {
            break;
        }
    }
    Some(out)
}
