//! The types a JavaScript file writes in JSDoc (#347): `@type`, `@param`, `@returns`, and the
//! fields of a `@typedef {Object}`. The caller reads them in `.js`, `.jsx`, `.mjs` and `.cjs`
//! files only ([`jsdoc_file`]): a `.ts` file writes annotations.

use std::ops::Range;
use std::path::Path;

use regex::Regex;

use super::*;

/// Whether `path` is a JavaScript file, whose types JSDoc writes.
pub fn jsdoc_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| matches!(e.to_str(), Some("js" | "jsx" | "mjs" | "cjs")))
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

/// Each `@tag {T} rest` of the JSDoc lines `doc`: the tag, `T`, and the rest of its line.
fn tags(doc: &[&str]) -> Vec<(String, String, String)> {
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
        out.push((tag, ty.to_owned(), rest.trim().to_owned()));
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

/// The lines of the JSDoc block `/** … */` that closes right above 0-based line `k`: none when a
/// blank line, code or a comment of another kind stands between them.
fn block_above(lines: &[&str], k: usize) -> Option<Range<usize>> {
    let end = k.checked_sub(1)?;
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

/// The type JSDoc writes for `name` bound on 1-based `line` of `text`: the `@type {T}` of a
/// `const`, `let` or `var` declaring it, on its line or right above it; else the `@param {T}
/// name` of the function whose header starts on the line.
pub fn jsdoc_binding(text: &str, line: usize, name: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let k = line.checked_sub(1)?;
    let own = lines.get(k)?;
    let declares = Regex::new(&format!(
        r"^\s*(?:/\*\*.*?\*/\s*)?(?:export\s+)?(?:const|let|var)\s+{}(?:[^\w$]|$)",
        regex::escape(name)
    ))
    .ok()?;
    // `/** @type {T} */ let x;` writes the type on the line itself.
    let inline = own
        .trim_start()
        .starts_with("/**")
        .then(|| own.split_once("*/").map(|(doc, _)| doc))
        .flatten();
    let doc: Vec<&str> = match (inline, block_above(&lines, k)) {
        (Some(doc), _) => vec![doc],
        (None, Some(block)) => lines[block].to_vec(),
        (None, None) => return None,
    };
    let tags = tags(&doc);
    if declares.is_match(own) {
        let (_, ty, _) = tags.iter().find(|(tag, ..)| tag == "type")?;
        return jsdoc_type(ty);
    }
    // `@param {T} [name=default]` names an optional one.
    let (_, ty, _) = tags
        .iter()
        .find(|(tag, _, rest)| tag == "param" && tag_name(rest) == Some(name))?;
    jsdoc_type(ty)
}

/// The `@returns {T}` or `@return {T}` of the JSDoc block right above the function or method
/// declared on 1-based `decl` of `text`.
pub fn jsdoc_returns(text: &str, decl: usize) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let block = block_above(&lines, decl.checked_sub(1)?)?;
    let tags = tags(&lines[block]);
    let (_, ty, _) = tags
        .iter()
        .find(|(tag, ..)| tag == "returns" || tag == "return")?;
    jsdoc_type(ty)
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

/// The name of the `@typedef {Object}` whose block holds the `@property` on 0-based `k`.
pub fn jsdoc_owner_name<'a>(lines: &[&'a str], k: usize) -> Option<&'a str> {
    let i = jsdoc_owner(lines, k)?;
    typedef(lines[i]).map(|(_, name)| name)
}

/// The 0-based line of the `@typedef {Object}` whose block holds the `@property` on 0-based `k`.
pub fn jsdoc_owner(lines: &[&str], k: usize) -> Option<usize> {
    if !lines[k].contains("@prop") {
        return None;
    }
    for i in (0..=k).rev() {
        if let Some((ty, _)) = typedef(lines[i]) {
            return jsdoc_object(ty).then_some(i);
        }
        let t = lines[i].trim_start();
        if !t.starts_with('*') || (i < k && t.contains("*/")) {
            return None;
        }
    }
    None
}

/// The fields `name` the `@typedef {Object}` on 0-based line `k` declares: each `@property {T}
/// name` or `@property {T} [name]` of its block, with `T` when it is one plain name. `None` when
/// the line writes no such typedef.
pub fn jsdoc_properties(lines: &[&str], k: usize, name: &str) -> Option<Vec<Binding>> {
    typedef(lines[k]).filter(|(ty, _)| jsdoc_object(ty))?;
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(k + 1) {
        if typedef(l).is_some() || !l.trim_start().starts_with('*') {
            break;
        }
        if let [(tag, ty, rest)] = &tags(&[l])[..]
            && (tag == "property" || tag == "prop")
            && tag_name(rest) == Some(name)
        {
            let value = jsdoc_type(ty).map_or(Value::Unknown, Value::Type);
            out.push(Binding { line: i + 1, value });
        }
        if l.contains("*/") {
            break;
        }
    }
    Some(out)
}
