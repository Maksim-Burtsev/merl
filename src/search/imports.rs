//! What a file imports, and which file on disk an import names.

use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

/// The names a file binds by importing, each with the module path it comes from, as the parts
/// a file system would spell it in. `import numpy as np` binds `np` to `[numpy]`; `from json
/// import load` binds `load` to `[json, load]` (a module or a name in one, the caller relaxes
/// the path until a file matches). A relative import (`from ..models import X`, `./utils`) binds
/// too, with its dots as the leading part: it names a project file, never one outside. A
/// TypeScript path ends in what the import takes from the module: the name, `default`, or `*`
/// for the whole module (`* as ns`, `require`), and a `node:` module keeps the prefix that makes
/// it Node's own. Rust's in-crate `crate::` and `super::` paths are left out.
pub fn imports(kind: Kind, text: &str) -> Vec<(String, Vec<String>)> {
    // A Python import in a docstring's example binds nothing of the file.
    if kind == Kind::Python {
        let literal = literal_lines(kind, text);
        let code: Vec<&str> = text
            .lines()
            .zip(&literal)
            .map(|(l, inside)| if *inside { "" } else { l })
            .collect();
        return imports_as_written(kind, &code.join("\n"));
    }
    imports_as_written(kind, text)
}
/// The module a word in the module path of a Python import line names (#333), when byte `at` of
/// `line` is in that path: the path's parts up to and including the word. `from app.repos import
/// X` on `repos` is `[app, repos]`, `import a.b as c, d` on `a` is `[a]`, and a relative `from
/// ..x.y import z` on `x` keeps its dots as the first part, `["..", "x"]`. `None` on an imported
/// name, an alias, or any other line.
pub fn python_import_module(line: &str, at: usize) -> Option<Vec<String>> {
    static FROM: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*from\s+([\w.]+)\s+import\b").unwrap());
    static IMPORT: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^\s*import\s+([^#;]+)").unwrap());
    // Each module path the line spells, with where it starts.
    let paths: Vec<(usize, &str)> = match FROM.captures(line) {
        Some(c) => c
            .get(1)
            .map(|m| (m.start(), m.as_str()))
            .into_iter()
            .collect(),
        None => {
            let list = IMPORT.captures(line)?.get(1)?;
            let mut at = list.start();
            let mut out = Vec::new();
            for item in list.as_str().split(',') {
                let lead = item.len() - item.trim_start().len();
                if let Some(path) = item.split_whitespace().next() {
                    out.push((at + lead, path));
                }
                at += item.len() + 1;
            }
            out
        }
    };
    let (start, path) = paths
        .into_iter()
        .find(|&(s, p)| (s..s + p.len()).contains(&at))?;
    let end = path[at - start..]
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map_or(path.len(), |i| at - start + i);
    let spelled = &path[..end];
    let relative = spelled.trim_start_matches('.');
    let mut parts: Vec<String> = relative
        .split('.')
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect();
    if parts.is_empty() {
        return None;
    }
    if relative.len() < spelled.len() {
        parts.insert(0, ".".repeat(spelled.len() - relative.len()));
    }
    Some(parts)
}
/// A TypeScript `import … from`, `require` or `import x = require`: the clause (one of the first
/// three groups), the module, and after a `require(…)` what follows its `)`: a `.name` it takes,
/// and a last `(`, `[` or `.` when the value is used further (#328).
static TS_IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(
        r#"(?ms)^\s*(?:import\s+(?:type\s+)?([^'"]*?)\s*from\s*|(?:const|let|var)\s+([^=]+?)\s*=\s*(?:await\s+)?(?:require|import)\s*\(\s*|import\s+([\w$]+)\s*=\s*require\s*\(\s*)['"]([^'"]+)['"](\s*\)(?:\s*\.\s*[\w$]+)?\s*[(\[.]?)?"#,
    )
    .unwrap()
});
/// `text` with each declarator on a continuation line of a TypeScript `const`, `let` or `var`
/// statement written as a statement of its own, `var more = require("./m"),`, so that
/// [`TS_IMPORT`] reads it (#328). The lines stay where they are.
fn ts_continued(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.contains("require") {
        return text.into();
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<String> = lines.iter().map(|l| (*l).to_owned()).collect();
    let mut changed = false;
    for k in 0..lines.len() {
        for (l, _) in ts_declarators(&lines, k)
            .unwrap_or_default()
            .into_iter()
            .skip(1)
        {
            if l > k && !out[l].trim_start().starts_with("var ") {
                out[l] = format!("var {}", lines[l].trim_start());
                changed = true;
            }
        }
    }
    match changed {
        true => out.join("\n").into(),
        false => text.into(),
    }
}
/// The 1-based line of the TypeScript import in `text` that binds `name`, as [`imports`] reads
/// it: in an import wrapped over several lines, the line the name is written on.
pub fn ts_import_line(text: &str, name: &str) -> Option<usize> {
    ts_import_lines(text, name).first().copied()
}
/// Every line [`ts_import_line`] could name: each import of `text` that binds `name`.
pub fn ts_import_lines(text: &str, name: &str) -> Vec<usize> {
    let ident = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$');
    let text = ts_continued(text);
    let text = text.as_ref();
    TS_IMPORT
        .captures_iter(text)
        .filter_map(|c| {
            let binds = imports_as_written(Kind::TsJs, &c[0])
                .iter()
                .any(|(n, _)| n == name);
            let clause = c.get(1).or_else(|| c.get(2)).or_else(|| c.get(3))?;
            let s = clause.as_str();
            let at = s.match_indices(name).map(|(i, _)| i).find(|&i| {
                !ident(s[..i].chars().next_back()) && !ident(s[i + name.len()..].chars().next())
            });
            let at = at.filter(|_| binds)?;
            Some(text[..clause.start() + at].matches('\n').count() + 1)
        })
        .collect()
}
/// [`imports`] over every line of `text`, a docstring's too: what a reader inside the docstring's
/// example goes by.
pub fn imports_as_written(kind: Kind, text: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let parts = |module: &str, sep: &str| -> Vec<String> {
        module
            .split(sep)
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect()
    };
    // `name`, or `name as alias`: the alias is what the file uses.
    let bound = |item: &str| -> Option<(String, String)> {
        let mut it = item.split_whitespace();
        let name = it
            .next()?
            .trim_matches(|c| c == '{' || c == '}' || c == ',')
            .to_owned();
        let alias = match (it.next(), it.next()) {
            (Some("as"), Some(a)) => a.trim_end_matches(',').to_owned(),
            _ => name.clone(),
        };
        (!name.is_empty()).then_some((alias, name))
    };
    match kind {
        Kind::Python => {
            static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(
                    r"(?m)^\s*(?:from\s+([\w.]+)\s+import\s+(\((?:[^)#]|#[^\n]*)*\)|[^\n]+)|import\s+([^\n]+))",
                )
                .unwrap()
            });
            for c in IMPORT.captures_iter(text) {
                if let (Some(module), Some(names)) = (c.get(1), c.get(2)) {
                    let module = module.as_str();
                    let relative = module.trim_start_matches('.');
                    let mut base = parts(relative, ".");
                    if relative.len() < module.len() {
                        base.insert(0, ".".repeat(module.len() - relative.len()));
                    }
                    // A comment is no name, whatever commas or brackets it holds (#280).
                    let names: Vec<&str> = names
                        .as_str()
                        .lines()
                        .map(|l| l.split('#').next().unwrap_or_default())
                        .collect();
                    for item in names
                        .join("\n")
                        .trim()
                        .trim_matches(|c| c == '(' || c == ')')
                        .split(',')
                    {
                        if let Some((alias, name)) = bound(item.trim()) {
                            let mut path = base.clone();
                            path.push(name);
                            out.push((alias, path));
                        }
                    }
                } else if let Some(list) = c.get(3) {
                    // Nor after a plain `import` (#298).
                    let list = list.as_str().split('#').next().unwrap_or_default();
                    for item in list.split(',') {
                        if let Some((alias, name)) = bound(item.trim()) {
                            // `import a.b.c` binds `a`; `import a.b.c as d` binds `d` to `a.b.c`.
                            if alias == name {
                                let first = name.split('.').next().unwrap_or(&name).to_owned();
                                out.push((first.clone(), vec![first]));
                            } else {
                                out.push((alias, parts(&name, ".")));
                            }
                        }
                    }
                }
            }
        }
        // Rust's in-crate `crate::` and `super::` paths are [`rust_use_files`]'s.
        Kind::Rust => {
            out.extend(
                rust_uses(text)
                    .into_iter()
                    .filter(|(_, p, _)| !matches!(p[0].as_str(), "crate" | "super"))
                    .map(|(name, path, _)| (name, path)),
            );
        }
        Kind::Go => {
            static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r#"(?m)^\s*(?:import\s+)?(?:(\w+|\.|_)\s+)?"([^"]+)"\s*$"#).unwrap()
            });
            let version =
                |p: &str| p.starts_with('v') && p[1..].chars().all(|c| c.is_ascii_digit());
            for c in IMPORT.captures_iter(text) {
                let path = parts(&c[2], "/");
                if let Some(alias) = c.get(1) {
                    out.push((alias.as_str().to_owned(), path));
                    continue;
                }
                // The package name is the last element that is not a major version, without the
                // decorations a module path carries: `yaml.v3`, `nats.go`, `go-sqlite3`,
                // `bar-go`. A major version last can be the package itself
                // (`k8s.io/api/core/v1`), so it binds as well.
                let Some(last) = path.iter().rev().find(|p| !version(p)) else {
                    continue;
                };
                let mut name = last.as_str();
                if let Some((stem, suffix)) = name.rsplit_once('.')
                    && (version(suffix) || suffix == "go")
                {
                    name = stem;
                }
                name = name.strip_prefix("go-").unwrap_or(name);
                name = name.strip_suffix("-go").unwrap_or(name);
                out.push((name.to_owned(), path.clone()));
                if let Some(v) = path.last().filter(|p| version(p)) {
                    out.push((v.clone(), path.clone()));
                }
            }
        }
        Kind::TsJs => {
            let text = ts_continued(text);
            for c in TS_IMPORT.captures_iter(&text) {
                let module = &c[4];
                // `require("debug")("app")` is what the module's value returns, no import of it;
                // `require("./x").Name` takes the module's `Name` (#328).
                let after = c.get(5).map_or("", |m| m.as_str()).trim_end();
                if after.ends_with(['(', '[', '.']) {
                    continue;
                }
                let member = after.split_once('.').map(|(_, m)| m.trim());
                // `./x` and `../x` keep their dots as the first part; an absolute path gets one.
                let mut path = parts(module, "/");
                if module.starts_with('/') {
                    path.insert(0, ".".to_owned());
                }
                // A bare `x` is the default export after `import`, the whole module after
                // `require`. `x`, `* as x`, `{a, type b as c}` and `x, {a}` in one clause.
                let (clause, whole) = match (c.get(1), c.get(2).or_else(|| c.get(3))) {
                    (Some(m), _) => (m.as_str(), "default"),
                    (None, m) => (m.map_or("", |m| m.as_str()), member.unwrap_or("*")),
                };
                // What a destructuring takes out of a member is not read.
                if member.is_some() && clause.contains('{') {
                    continue;
                }
                let (outside, named) = clause.split_once('{').map_or((clause, ""), |(o, n)| {
                    (o, n.split('}').next().unwrap_or(""))
                });
                let with = |taken: &str| {
                    let mut p = path.clone();
                    p.push(taken.to_owned());
                    p
                };
                for item in outside.split(',').map(str::trim).filter(|i| !i.is_empty()) {
                    match item.strip_prefix("* as ") {
                        Some(ns) => out.push((ns.trim().to_owned(), with("*"))),
                        None => out.push((item.to_owned(), with(whole))),
                    }
                }
                for item in named.split(',') {
                    let item = item.trim();
                    let item = item.strip_prefix("type ").unwrap_or(item);
                    // A destructuring renames with `:`, `const { a: b } = require("./x")` (#328).
                    if let Some((name, alias)) = item.split_once(':') {
                        let (name, alias) = (name.trim(), alias.trim());
                        if !name.is_empty() && !alias.is_empty() {
                            out.push((alias.to_owned(), with(name)));
                        }
                        continue;
                    }
                    if let Some((alias, name)) = bound(item) {
                        out.push((alias, with(&name)));
                    }
                }
            }
        }
        // A `use` names one class, function or constant, and PSR-4 spells a namespace the way a
        // file system does, so the path is the name split on `\`: `use Illuminate\Support\Str`
        // binds `Str` to `vendor/…/Illuminate/Support/Str.php`. In column zero only — indented,
        // `use` pulls a trait into a class body and names no file — and a group `use A\{B, C}`
        // is left out, since one clause then binds several.
        Kind::Php => {
            static USE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"(?m)^use\s+(?:function\s+|const\s+)?([^;{]+);").unwrap()
            });
            for c in USE.captures_iter(text) {
                for item in c[1].split(',') {
                    if let Some((alias, name)) = bound(item.trim()) {
                        let path = parts(&name, "\\");
                        if let Some(last) = path.last().cloned() {
                            out.push((if alias == name { last } else { alias }, path));
                        }
                    }
                }
            }
        }
        // Nothing to bind without roots to resolve an `import` or a `require` against. A C
        // `#include` binds no name of its own either: it pastes a file in, and everything the
        // file declares is then visible unqualified, and a C# `using` opens a whole namespace
        // the same way. Zig's `const std = @import("std")` does bind one, but `std` is the root
        // itself, not a directory inside it, so narrowing by it would find nothing.
        Kind::Jvm
        | Kind::Ruby
        | Kind::C
        | Kind::CSharp
        | Kind::Swift
        | Kind::Lua
        | Kind::Elixir
        | Kind::Zig
        | Kind::Proto
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml
        | Kind::Markdown
        // `#import "./parts.graphql"` pastes the file in and binds no name: [`graphql_import`].
        | Kind::Graphql => {}
    }
    out
}
/// Whether 1-based `line` of `text`, a TypeScript line ending in `name<`, declares a method: the
/// `>` that closes the type parameters is followed by a parameter list and then a body or a
/// return type. prettier writes a call with long type arguments the same way, and its
/// `>(…)` ends in `);` or goes on as an expression.
pub fn declares_wrapped_generic(text: &str, line: usize) -> bool {
    let lines: Vec<&str> = text.lines().collect();
    let Some(k) = line.checked_sub(1).filter(|&k| k < lines.len()) else {
        return false;
    };
    let ind = indent(lines[k]);
    let closer = (k + 1..lines.len().min(k + 40))
        .find(|&i| !lines[i].trim().is_empty() && indent(lines[i]) <= ind);
    let Some(i) = closer.filter(|&i| lines[i].trim_start().starts_with(">(")) else {
        return false;
    };
    let open = lines[i].find('(').unwrap_or(0);
    group(Kind::TsJs, &lines, i, open).is_some_and(|(_, _, rest)| {
        let rest = rest.trim();
        rest.starts_with(':') || rest.starts_with('{')
    })
}
/// The name `export default Name;` hands out, when it is a bare name (#335).
pub fn default_name(line: &str) -> Option<String> {
    static DEFAULT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"^\s*export\s+default\s+([A-Za-z_$][\w$]*)\s*;?\s*$").unwrap()
    });
    let name = DEFAULT.captures(line)?[1].to_owned();
    (!matches!(name.as_str(), "class" | "function" | "async" | "abstract")).then_some(name)
}
/// What a CommonJS module hands out as a whole (#328): the 1-based line of its one
/// `module.exports = …`, and the name it assigns when that is a bare name, as `module.exports =
/// Segment;` does. The outer `None` is a module that says nothing of `module.exports` or
/// `exports`, an ES module; the inner one a module that assigns `module.exports` more than once or
/// builds it with `exports.x = …` lines, where nothing is known of the whole.
pub fn module_exports(text: &str) -> Option<Option<(usize, Option<String>)>> {
    static WHOLE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^[ \t]*module\.exports\s*=\s*([^=\n][^\n]*)$").unwrap()
    });
    static PART: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?m)^[ \t]*(?:module\.)?exports\.[\w$]+\s*=[^=]").unwrap()
    });
    static NAME: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"^([A-Za-z_$][\w$]*)\s*;?\s*$").unwrap());
    let whole: Vec<regex::Captures> = WHOLE.captures_iter(text).collect();
    let part = PART.is_match(text);
    if whole.is_empty() && !part {
        return None;
    }
    Some(match (whole.as_slice(), part) {
        ([one], false) => {
            let line = text[..one.get(0).unwrap().start()].matches('\n').count() + 1;
            let name = NAME.captures(one[1].trim()).map(|c| c[1].to_owned());
            Some((line, name))
        }
        _ => None,
    })
}
/// A TypeScript method header's start, up to its `(`, for [`ts_call_statement`]: `word(`,
/// `async word<T>(`, `word?(`.
pub fn ts_method_head(word: &str) -> Regex {
    Regex::new(&format!(
        r"^\s*(?:(?:public|private|protected|static|readonly|abstract|override|async|get|set)\s+)*\*?{}\??\s*(?:<.*?>)?\(",
        regex::escape(word)
    ))
    .expect("an escaped name keeps the pattern valid")
}
/// Whether 1-based `line`, a TypeScript line shaped like a method header `word(…`, is
/// a call statement instead (#343): one of its arguments is what no parameter is (a
/// string, a number, a bracket, a callback, `this.x`), or, wrapped after its `(`, the line that
/// closes it, back at its indent, goes on as no body and no return type (`);`, `),`, `).x`), as
/// [`declares_wrapped_generic`] tells for `word<`.
///
/// `head` is [`ts_method_head`] of the word, `l` the line, and `text` reads the file, only for a
/// line that ends in its `(`.
pub fn ts_call_statement(
    head: &Regex,
    l: &str,
    line: usize,
    text: impl FnOnce() -> Option<String>,
) -> bool {
    let Some(m) = head.find(l) else {
        return false;
    };
    let rest = l[m.end()..].trim_start();
    if rest.is_empty() {
        let Some(text) = text() else {
            return false;
        };
        let lines: Vec<&str> = text.lines().collect();
        let ind = indent(l);
        let closer = (line..lines.len().min(line + 60))
            .find(|&i| !lines[i].trim().is_empty() && indent(lines[i]) <= ind);
        return closer.is_some_and(|i| {
            lines[i]
                .trim_start()
                .strip_prefix(')')
                .is_some_and(|after| {
                    let after = after.trim_start();
                    !after.starts_with('{') && !after.starts_with(':')
                })
        });
    }
    // Each parameter opens with a name, a pattern, a rest or the list's end: an argument that
    // is a string, a number, a callback or `this.x` makes it a call.
    !split_top(Kind::TsJs, rest, b',').iter().all(|item| {
        let item = item.trim();
        if item.is_empty() || item.starts_with([')', '{', '[', '@']) || item.starts_with("...") {
            return true;
        }
        let name = item
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
            .map_or(item, |n| &item[..n]);
        if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
            return false;
        }
        let after = item[name.len()..].trim_start();
        // `this: Window` and `x?: T` are parameters, whatever the name; a modifier opens one.
        if after.starts_with([':', '?'])
            || matches!(
                name,
                "public" | "private" | "protected" | "readonly" | "override"
            )
        {
            return true;
        }
        let keyword = matches!(
            name,
            "function"
                | "async"
                | "new"
                | "await"
                | "this"
                | "null"
                | "undefined"
                | "true"
                | "false"
                | "typeof"
                | "void"
                | "yield"
                | "class"
                | "super"
        );
        !keyword
            && (after.is_empty()
                || after.starts_with(')')
                || (after.starts_with('=') && !after.starts_with("=>") && !after.starts_with("==")))
    })
}
/// The name a TypeScript module declares what it exports as `name` under: `Hono` for
/// `export { Hono as HonoBase }`. A re-export `… from "./x"` declares nothing here.
pub fn exported_as(text: &str, name: &str) -> Option<String> {
    static EXPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?m)^\s*export\s+(?:type\s+)?\{([^}]*)\}\s*(from\b)?"#).unwrap()
    });
    EXPORT
        .captures_iter(text)
        .filter(|c| c.get(2).is_none())
        .find_map(|c| {
            c[1].split(',').find_map(|item| {
                let item = item.trim();
                let (local, exported) = item
                    .strip_prefix("type ")
                    .unwrap_or(item)
                    .split_once(" as ")?;
                (exported.trim() == name).then(|| local.trim().to_owned())
            })
        })
}
/// The modules a TypeScript barrel hands `name` on from, as [`imports`] spells a module:
/// `export * from "./a"` and `export { name } from "./a"`. Under another name
/// (`export { x as name }`) nothing is followed: [`reexported`] has those.
pub fn reexports(text: &str, name: &str) -> Vec<Vec<String>> {
    reexported(text, name)
        .into_iter()
        .filter(|(_, taken)| taken == name)
        .map(|(module, _)| module)
        .collect()
}
/// What a TypeScript barrel hands `name` on from (#335): each module, as [`imports`] spells one,
/// with what it takes there, as an import path ends: `name` for `export { name }` and `export *`,
/// `x` for `export { x as name }`, `default` for `export { default as name }`.
pub fn reexported(text: &str, name: &str) -> Vec<(Vec<String>, String)> {
    static EXPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?m)^\s*export\s+(?:type\s+)?(\*|\{[^}]*\})\s*from\s*['"]([^'"]+)['"]"#)
            .unwrap()
    });
    EXPORT
        .captures_iter(text)
        .filter_map(|c| {
            let taken = match &c[1] {
                "*" => name.to_owned(),
                items => items.trim_matches(['{', '}']).split(',').find_map(|i| {
                    let i = i.trim();
                    let i = i.strip_prefix("type ").unwrap_or(i);
                    match i.split_once(" as ") {
                        Some((local, exported)) => {
                            (exported.trim() == name).then(|| local.trim().to_owned())
                        }
                        None => (i == name).then(|| name.to_owned()),
                    }
                })?,
            };
            let mut path: Vec<String> = c[2]
                .split('/')
                .filter(|p| !p.is_empty())
                .map(str::to_owned)
                .collect();
            if c[2].starts_with('/') {
                path.insert(0, ".".to_owned());
            }
            Some((path, taken))
        })
        .collect()
}
/// The 1-based line of the Go file `text` whose import binds `name`, as [`imports`] reads it.
pub fn go_import_line(text: &str, name: &str) -> Option<usize> {
    let binds = |l: &str| imports(Kind::Go, l).iter().any(|(n, _)| n == name);
    text.lines().position(binds).map(|i| i + 1)
}
/// What every `use` of the Rust file `text` binds, as [`imports`] reads it, with the in-crate
/// `crate::` and `super::` paths kept, and whether the `use` starts in column zero: at the top
/// of the file, not in a function or an inline `mod`.
fn rust_uses(text: &str) -> Vec<(String, Vec<String>, bool)> {
    static USE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?ms)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+([^;]+);").unwrap()
    });
    let mut out = Vec::new();
    for c in USE.captures_iter(text) {
        let at = c
            .get(0)
            .map_or(0, |m| m.end() - m.as_str().trim_start().len());
        let top = at == 0 || text[..at].ends_with('\n');
        let mut bound = Vec::new();
        use_tree(&c[1], &[], &mut bound);
        out.extend(bound.into_iter().map(|(name, path)| (name, path, top)));
    }
    out
}
/// The files of the crate module a `use crate::…` or `use super::…` of the Rust file `here`
/// takes `name` from: `src/store.rs` or `src/store/mod.rs` for `use crate::store::Cache;`, the
/// crate root's `lib.rs` and `main.rs` for `use crate::Cache;`. Empty when the file binds `name`
/// otherwise, more than once or only inside a function or an inline `mod` (whose `super` is not
/// the file's), or when `here` is not in the `src/` of a `Cargo.toml` (a binary
/// under `src/bin/` is a crate of its own). Only the paths a module's file sits at by default
/// are tried: an inline `mod` or a `#[path]` leaves the caller nothing to prove.
pub fn rust_use_files(files: &[PathBuf], here: &Path, text: &str, name: &str) -> Vec<PathBuf> {
    let mut bound = rust_uses(text).into_iter().filter(|(n, _, _)| n == name);
    let (Some((_, path, true)), None) = (bound.next(), bound.next()) else {
        return Vec::new();
    };
    let Some(src) = here.ancestors().skip(1).find(|d| {
        d.file_name().is_some_and(|n| n == "src")
            && files.contains(&d.parent().unwrap_or(Path::new("")).join("Cargo.toml"))
    }) else {
        return Vec::new();
    };
    let Ok(inside) = here.strip_prefix(src) else {
        return Vec::new();
    };
    let mut module: Vec<String> = inside
        .with_extension("")
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    if module.first().is_some_and(|m| m == "bin") {
        return Vec::new();
    }
    if matches!(module.as_slice(), [m] if m == "lib" || m == "main")
        || module.last().is_some_and(|m| m == "mod")
    {
        module.pop();
    }
    let Some((_, parts)) = path.split_last() else {
        return Vec::new();
    };
    let parts = match parts.split_first() {
        Some((root, rest)) if root == "crate" => {
            module.clear();
            rest
        }
        Some((root, _)) if root == "super" => {
            let supers = parts.iter().take_while(|p| *p == "super").count();
            if supers > module.len() {
                return Vec::new();
            }
            module.truncate(module.len() - supers);
            &parts[supers..]
        }
        _ => return Vec::new(),
    };
    module.extend(parts.iter().cloned());
    let at = module.iter().fold(src.to_path_buf(), |p, m| p.join(m));
    let wanted = match module.is_empty() {
        true => vec![src.join("lib.rs"), src.join("main.rs")],
        false => vec![at.with_extension("rs"), at.join("mod.rs")],
    };
    wanted.into_iter().filter(|f| files.contains(f)).collect()
}
/// One `use` tree: `a::b::{c, d as e, f::*}` binds `c`, `e` and every name of `f`.
fn use_tree(tree: &str, prefix: &[String], out: &mut Vec<(String, Vec<String>)>) {
    let tree = tree.trim();
    if tree.is_empty() {
        return;
    }
    if let Some((head, rest)) = tree.split_once('{') {
        let rest = rest.trim_end().trim_end_matches('}');
        let mut path = prefix.to_vec();
        path.extend(
            head.split("::")
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(String::from),
        );
        // Split the list at the commas outside nested braces.
        let (mut depth, mut start) = (0, 0);
        for (i, ch) in rest.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                ',' if depth == 0 => {
                    use_tree(&rest[start..i], &path, out);
                    start = i + 1;
                }
                _ => {}
            }
        }
        use_tree(&rest[start..], &path, out);
        return;
    }
    let mut path = prefix.to_vec();
    let (tree, alias) = tree
        .split_once(" as ")
        .map_or((tree, None), |(t, a)| (t.trim(), Some(a.trim().to_owned())));
    path.extend(
        tree.split("::")
            .map(str::trim)
            .filter(|p| !p.is_empty() && *p != "self" && *p != "*")
            .map(String::from),
    );
    let Some(last) = path.last().cloned() else {
        return;
    };
    out.push((alias.unwrap_or(last), path));
}
/// The project files of the module an import in `here` spells as `module`, the parts
/// [`imports`] gives without what a TypeScript import takes from it; empty when the module is not
/// the project's. Paths are relative to `root`, and only what the project walk listed in `files`
/// counts.
///
/// - Python: `a/b.py` or the package `a/b/__init__.py`. Relative to the directory of `here` behind
///   dots, one directory up per dot past the first; otherwise at any depth (the root, `src/`, a
///   folder of a monorepo) but not inside a package, since `json` is not `myapp/json.py`.
/// - TypeScript: `./x` from the directory of `here`, an alias from the `paths` of the nearest
///   `tsconfig.json` (or `jsconfig.json`) or a name under its `baseUrl`, as `x.ts`, `x.tsx`, `x.d.ts`, the JavaScript
///   forms, then `x/index.*`. `./x.js` is `x.ts` first, as ESM projects write it.
/// - Go: the directory under the `go.mod` whose `module` the import path starts with (the longest,
///   for a module nested in another), without its `_test.go` files.
pub fn module_files(
    kind: Kind,
    root: &Path,
    files: &[PathBuf],
    here: &Path,
    module: &[String],
) -> Vec<PathBuf> {
    let dir = here.parent().unwrap_or(Path::new(""));
    match kind {
        Kind::Python => {
            let (base, parts) = match module.split_first() {
                Some((dots, rest)) if dots.starts_with('.') => {
                    match dir.ancestors().nth(dots.len() - 1) {
                        Some(base) => (Some(base), rest),
                        None => return Vec::new(),
                    }
                }
                _ => (None, module),
            };
            let name: PathBuf = parts.iter().collect();
            let mut forms = vec![name.join("__init__.py")];
            if !parts.is_empty() {
                forms.insert(0, name.with_extension("py"));
            } else if base.is_none() {
                return Vec::new();
            }
            files
                .iter()
                .filter(|f| {
                    forms.iter().any(|m| match base {
                        Some(base) => **f == base.join(m),
                        None => {
                            f.ends_with(m)
                                && f.ancestors()
                                    .nth(m.components().count())
                                    .is_some_and(|top| !files.contains(&top.join("__init__.py")))
                        }
                    })
                })
                .cloned()
                .collect()
        }
        Kind::TsJs => {
            let listed: std::collections::HashSet<&Path> =
                files.iter().map(PathBuf::as_path).collect();
            let spec = module.join("/");
            let bases = if module.first().is_some_and(|p| p.starts_with('.')) {
                vec![dir.join(&spec)]
            } else {
                ts_aliases(root, dir, &spec)
            };
            const EXTENSIONS: [&str; 9] =
                ["ts", "tsx", "d.ts", "js", "jsx", "mts", "cts", "mjs", "cjs"];
            bases
                .iter()
                .filter_map(|b| lexical(b))
                .find_map(|base| {
                    let s = base.to_string_lossy();
                    let stem = [".js", ".jsx", ".mjs", ".cjs"]
                        .iter()
                        .find_map(|e| s.strip_suffix(e))
                        .unwrap_or(&s);
                    EXTENSIONS
                        .iter()
                        .map(|e| PathBuf::from(format!("{stem}.{e}")))
                        .chain([base.clone()])
                        .chain(EXTENSIONS.iter().map(|e| base.join(format!("index.{e}"))))
                        .find(|c| listed.contains(c.as_path()) && kind_of(c) == Some(Kind::TsJs))
                })
                .into_iter()
                .collect()
        }
        Kind::Go => {
            let import = module.join("/");
            let package = files
                .iter()
                .filter(|f| f.file_name().is_some_and(|n| n == "go.mod"))
                .filter_map(|gomod| {
                    let text = std::fs::read_to_string(root.join(gomod)).ok()?;
                    let name = text
                        .lines()
                        .find_map(|l| l.trim().strip_prefix("module "))?
                        .split_whitespace()
                        .next()?
                        .trim_matches('"')
                        .to_owned();
                    let rest = match import.strip_prefix(&name)? {
                        "" => "",
                        rest => rest.strip_prefix('/')?,
                    };
                    Some((name.len(), gomod.parent()?.join(rest)))
                })
                .max_by_key(|(n, _)| *n);
            let Some((_, package)) = package else {
                return Vec::new();
            };
            files
                .iter()
                .filter(|f| f.parent() == Some(package.as_path()))
                .filter(|f| {
                    kind_of(f) == Some(Kind::Go) && !f.to_string_lossy().ends_with("_test.go")
                })
                .cloned()
                .collect()
        }
        Kind::Rust
        | Kind::Jvm
        | Kind::Ruby
        | Kind::C
        | Kind::CSharp
        | Kind::Swift
        | Kind::Php
        | Kind::Lua
        | Kind::Elixir
        | Kind::Zig
        | Kind::Proto
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml
        | Kind::Markdown => Vec::new(),
        // The path of an `#import`, relative to the importing file.
        Kind::Graphql => lexical(&dir.join(module.join("/")))
            .filter(|f| files.contains(f))
            .into_iter()
            .collect(),
    }
}
/// The path of the `#import "./parts.graphql"` a GraphQL `line` is, when byte `col` stands on it
/// or its quotes: the convention of `graphql-tag/loader`, which pastes that file in.
pub fn graphql_import(line: &str, col: usize) -> Option<&str> {
    static IMPORT: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#"^#\s*import\s+(["'])([^"']+)["']"#).unwrap());
    let c = IMPORT.captures(line)?;
    let path = c.get(2)?;
    (c.get(1)?.start() <= col && col <= path.end()).then_some(path.as_str())
}
/// Where an aliased TypeScript specifier points, most specific first: the targets of the
/// `compilerOptions.paths` entries it matches, then the specifier under `baseUrl`. Read from the
/// `tsconfig.json`, or a JavaScript project's `jsconfig.json`, nearest above `dir` and the configs it `extends` by a relative path, the
/// nearest setting winning; `paths` are relative to `baseUrl` when there is one, else to the
/// config that declares them. Comments and trailing commas are fine: only these keys are read.
fn ts_aliases(root: &Path, dir: &Path, spec: &str) -> Vec<PathBuf> {
    let (paths, url) = ts_config(root, dir, spec);
    paths.into_iter().chain(url.map(|u| u.join(spec))).collect()
}
/// Whether an entry of the `compilerOptions.paths` [`ts_aliases`] reads matches `spec`, or
/// `baseUrl` has a file or a directory of its first part: the project's own module, no package.
pub(super) fn ts_alias(root: &Path, files: &[PathBuf], dir: &Path, spec: &str) -> bool {
    let (paths, url) = ts_config(root, dir, spec);
    let first = spec.split('/').next().unwrap_or(spec);
    !paths.is_empty()
        || url.is_some_and(|u| {
            let base = u.join(first);
            files
                .iter()
                .any(|f| f.starts_with(&base) || f.with_extension("") == base)
        })
}
/// The targets of the `paths` entries `spec` matches, most specific first, and `baseUrl`.
fn ts_config(root: &Path, dir: &Path, spec: &str) -> (Vec<PathBuf>, Option<PathBuf>) {
    static COMMENT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#""(?:[^"\\]|\\.)*"|//[^\n]*|/\*(?s:.*?)\*/"#).unwrap()
    });
    let key = |name: &str, value: &str| {
        Regex::new(&format!(r#""{name}"\s*:\s*{value}"#)).expect("a fixed key pattern")
    };
    let (base_url, paths, extends) = (
        key("baseUrl", r#""([^"]*)""#),
        key("paths", r"\{([^}]*)\}"),
        key("extends", r#""(\.[^"]*)""#),
    );
    let entry = Regex::new(r#""([^"]+)"\s*:\s*\[([^\]]*)\]"#).expect("a fixed pattern");
    let string = Regex::new(r#""([^"]*)""#).expect("a fixed pattern");

    let mut config = dir
        .ancestors()
        .flat_map(|d| [d.join("tsconfig.json"), d.join("jsconfig.json")])
        .find(|c| root.join(c).is_file());
    let (mut url, mut table): (Option<PathBuf>, Option<(PathBuf, String)>) = (None, None);
    // ponytail: eight `extends` hops, which also ends a cycle.
    for _ in 0..8 {
        let Some(file) = config.take() else { break };
        let Ok(text) = std::fs::read_to_string(root.join(&file)) else {
            break;
        };
        let text = COMMENT.replace_all(&text, |c: &regex::Captures| {
            if c[0].starts_with('"') {
                c[0].to_owned()
            } else {
                String::new()
            }
        });
        let at = file.parent().unwrap_or(Path::new("")).to_path_buf();
        if url.is_none() {
            url = base_url.captures(&text).map(|c| at.join(&c[1]));
        }
        if table.is_none() {
            table = paths.captures(&text).map(|c| (at.clone(), c[1].to_owned()));
        }
        // `"extends": "./base"` is `./base.json`.
        config = extends.captures(&text).map(|c| {
            if c[1].ends_with(".json") {
                at.join(&c[1])
            } else {
                at.join(format!("{}.json", &c[1]))
            }
        });
    }
    let mut targets: Vec<(usize, PathBuf)> = Vec::new();
    if let Some((at, table)) = &table {
        let from = url.as_ref().unwrap_or(at);
        for e in entry.captures_iter(table) {
            // An exact key outranks every wildcard; among wildcards the longer prefix wins.
            let matched = match e[1].split_once('*') {
                Some((pre, post)) => spec
                    .strip_prefix(pre)
                    .and_then(|s| s.strip_suffix(post))
                    .map(|s| (pre.len(), s)),
                None => (e[1] == *spec).then_some((usize::MAX, "")),
            };
            if let Some((rank, star)) = matched {
                targets.extend(
                    string
                        .captures_iter(&e[2])
                        .map(|t| (rank, from.join(t[1].replace('*', star)))),
                );
            }
        }
    }
    targets.sort_by_key(|(rank, _)| std::cmp::Reverse(*rank));
    (targets.into_iter().map(|(_, t)| t).collect(), url)
}
/// `path` with its `.` and `..` parts folded away, `None` when it climbs above where it starts.
fn lexical(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            c => out.push(c),
        }
    }
    Some(out)
}

/// Whether `path` is (in) the module spelled by `parts`: every part is a directory or file
/// stem on it, in order. A package directory carries a version (`regex-1.11.1`,
/// `toml@v1.2.3`), a Go module escapes upper case (`!burnt!sushi`) and a crate name spells
/// `_` as `-`: those are ignored. The types of `@scope/pkg` are `@types/scope__pkg`, read so when
/// the scope and the name are those.
pub fn in_module(path: &Path, parts: &[String]) -> bool {
    let want: Vec<String> = parts.iter().map(|p| module_part(p)).collect();
    let mut components = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .peekable();
    let mut i = 0;
    while let Some(c) = components.next() {
        let scoped = |scope: &str, pkg: &str, next: &str| {
            let rest = scope.strip_prefix('@').and_then(|s| next.strip_prefix(s));
            c == "@types" && rest.and_then(|r| r.strip_prefix("__")) == Some(pkg)
        };
        match &parts[i..] {
            [] => break,
            [scope, pkg, ..] if components.peek().is_some_and(|n| scoped(scope, pkg, n)) => {
                components.next();
                i += 2;
            }
            _ => i += usize::from(want[i] == module_part(&c)),
        }
    }
    i == parts.len()
}
/// A directory or a part of a module path without what only one of the two carries.
fn module_part(s: &str) -> String {
    let s = s.split('@').next().unwrap_or(s);
    // `fs.d.ts`, `python3.13`, `github.com`: the stem before every extension.
    let s = s.split('.').next().unwrap_or(s);
    // `name-1.2.3`: the version after the first `-` followed by a digit.
    let s = s
        .match_indices('-')
        .find(|(i, _)| s[i + 1..].starts_with(|c: char| c.is_ascii_digit()))
        .map_or(s, |(i, _)| &s[..i]);
    s.replace('!', "").replace('-', "_").to_ascii_lowercase()
}
/// The files among `files` of `module`, shortened from its end until some match, with how many
/// of its parts that left: `from json import load` is `json/load`, then `json`. A Go import of
/// `package` parts names one directory, so down to that length a file has to be in it
/// ([`in_package`]). `None` when not even the first part matches.
pub fn module_among<P: AsRef<Path> + Clone>(
    files: &[P],
    module: &[String],
    package: Option<usize>,
) -> Option<(usize, Vec<P>)> {
    (1..=module.len()).rev().find_map(|n| {
        let m = &module[..n];
        let found: Vec<P> = files
            .iter()
            .filter(|p| match package {
                Some(k) if n >= k => in_package(p.as_ref(), m),
                _ => in_module(p.as_ref(), m),
            })
            .cloned()
            .collect();
        (!found.is_empty()).then_some((n, found))
    })
}
/// Whether the Go file `path` is of the package imported as `parts` (#100): a Go package is one
/// directory, so the file's own directory ends with the import path, and `database/sql` is not
/// `database/sql/driver`. A path with no dot in its first part is the standard library's, which
/// sits right under GOROOT's `src` (or a `vendor` there): `errors` is not `github.com/pkg/errors`.
pub fn in_package(path: &Path, parts: &[String]) -> bool {
    let dirs: Vec<String> = path
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .map(|c| module_part(&c.as_os_str().to_string_lossy()))
        .collect();
    let want: Vec<String> = parts.iter().map(|p| module_part(p)).collect();
    let std = parts.first().is_some_and(|p| !p.contains('.'));
    dirs.strip_suffix(want.as_slice())
        .is_some_and(|above| !std || above.last().is_some_and(|d| d == "src" || d == "vendor"))
}
