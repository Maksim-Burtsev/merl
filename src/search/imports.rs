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
                    r"(?m)^\s*(?:from\s+([\w.]+)\s+import\s+(\([^)]*\)|[^\n]+)|import\s+([^\n]+))",
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
                    for item in names
                        .as_str()
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
                    for item in list.as_str().split(',') {
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
        Kind::Rust => {
            static USE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(r"(?ms)^\s*(?:pub(?:\([^)]*\))?\s+)?use\s+([^;]+);").unwrap()
            });
            for c in USE.captures_iter(text) {
                use_tree(&c[1], &[], &mut out);
            }
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
            static IMPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
                Regex::new(
                    r#"(?ms)^\s*(?:import\s+(?:type\s+)?([^'"]*?)\s*from\s*|(?:const|let|var)\s+([^=]+?)\s*=\s*(?:await\s+)?(?:require|import)\s*\(\s*|import\s+([\w$]+)\s*=\s*require\s*\(\s*)['"]([^'"]+)['"]"#,
                )
                .unwrap()
            });
            for c in IMPORT.captures_iter(text) {
                let module = &c[4];
                // `./x` and `../x` keep their dots as the first part; an absolute path gets one.
                let mut path = parts(module, "/");
                if module.starts_with('/') {
                    path.insert(0, ".".to_owned());
                }
                // A bare `x` is the default export after `import`, the whole module after
                // `require`. `x`, `* as x`, `{a, type b as c}` and `x, {a}` in one clause.
                let (clause, whole) = match (c.get(1), c.get(2).or_else(|| c.get(3))) {
                    (Some(m), _) => (m.as_str(), "default"),
                    (None, m) => (m.map_or("", |m| m.as_str()), "*"),
                };
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
                    if let Some((alias, name)) = bound(item.strip_prefix("type ").unwrap_or(item)) {
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
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml => {}
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
/// (`export { x as name }`) nothing is followed.
pub fn reexports(text: &str, name: &str) -> Vec<Vec<String>> {
    static EXPORT: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r#"(?m)^\s*export\s+(?:type\s+)?(\*|\{[^}]*\})\s*from\s*['"]([^'"]+)['"]"#)
            .unwrap()
    });
    EXPORT
        .captures_iter(text)
        .filter(|c| {
            let items = c[1].trim_matches(['{', '}']);
            &c[1] == "*"
                || items
                    .split(',')
                    .any(|i| i.trim().strip_prefix("type ").unwrap_or(i.trim()) == name)
        })
        .map(|c| {
            let mut path: Vec<String> = c[2]
                .split('/')
                .filter(|p| !p.is_empty())
                .map(str::to_owned)
                .collect();
            if c[2].starts_with('/') {
                path.insert(0, ".".to_owned());
            }
            path
        })
        .collect()
}
/// The 1-based line of the Go file `text` whose import binds `name`, as [`imports`] reads it.
pub fn go_import_line(text: &str, name: &str) -> Option<usize> {
    let binds = |l: &str| imports(Kind::Go, l).iter().any(|(n, _)| n == name);
    text.lines().position(binds).map(|i| i + 1)
}
/// One `use` tree: `a::b::{c, d as e, f::*}` binds `c`, `e` and every name of `f`. A `crate`,
/// `self` or `super` root is the project.
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
    if matches!(
        path.first().map(String::as_str),
        None | Some("crate" | "super")
    ) {
        return;
    }
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
///   `tsconfig.json` or a name under its `baseUrl`, as `x.ts`, `x.tsx`, `x.d.ts`, the JavaScript
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
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml => Vec::new(),
    }
}
/// Where an aliased TypeScript specifier points, most specific first: the targets of the
/// `compilerOptions.paths` entries it matches, then the specifier under `baseUrl`. Read from the
/// `tsconfig.json` nearest above `dir` and the configs it `extends` by a relative path, the
/// nearest setting winning; `paths` are relative to `baseUrl` when there is one, else to the
/// config that declares them. Comments and trailing commas are fine: only these keys are read.
fn ts_aliases(root: &Path, dir: &Path, spec: &str) -> Vec<PathBuf> {
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
        .map(|d| d.join("tsconfig.json"))
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
    targets
        .into_iter()
        .map(|(_, t)| t)
        .chain(url.map(|u| u.join(spec)))
        .collect()
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
/// `_` as `-`: those are ignored. An npm scope is spelled out, and the types of `@scope/pkg`
/// are `@types/scope__pkg`.
pub fn in_module(path: &Path, parts: &[String]) -> bool {
    let mut i = 0;
    let mut types = false;
    for c in path.components() {
        let c = c.as_os_str().to_string_lossy();
        let scoped = |scope: &str, pkg: &str| {
            types
                && scope
                    .strip_prefix('@')
                    .is_some_and(|s| *c == format!("{s}__{pkg}"))
        };
        i += match &parts[i..] {
            [] => break,
            [scope, pkg, ..] if scoped(scope, pkg) => 2,
            [scope, ..] if scope.starts_with('@') => usize::from(*c == **scope),
            [part, ..] => usize::from(module_part(part) == module_part(&c)),
        };
        types = c == "@types";
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
