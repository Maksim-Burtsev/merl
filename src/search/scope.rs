//! Where a definition may live: the project's own scope, the roots the toolchain on this
//! machine adds outside it, the files under them, which of them a platform builds, and
//! the order they are offered in.

use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

/// Whether `path` is searched for a definition asked for in `here`, a file of `kind`. Stages,
/// anchors, compose services and CI jobs belong to their file; a Terraform module is a
/// directory; everything else is every file of the same kind, so `.tsx` finds `.ts`.
pub fn in_def_scope(kind: Kind, here: &Path, path: &Path) -> bool {
    match kind {
        Kind::Docker | Kind::Yaml | Kind::Markdown | Kind::Html => path == here,
        Kind::Terraform => kind_of(path) == Some(kind) && path.parent() == here.parent(),
        Kind::Python
        | Kind::Go
        | Kind::Rust
        | Kind::TsJs
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
        | Kind::PowerShell
        | Kind::Dart
        | Kind::Cmake
        | Kind::Nix
        | Kind::Starlark
        | Kind::Sql
        | Kind::Make
        | Kind::Graphql
        | Kind::Css => kind_of(path) == Some(kind),
    }
}
/// Where the standard library and the dependencies of the project at `root` live on this
/// machine, for a file of `kind`; empty when the toolchain is not installed. Each is asked the
/// way it answers itself: the project's `.venv`, or `sys.path` of the `python3` on the PATH,
/// `rustc --print sysroot` plus the registry crates `Cargo.lock` names, `GOROOT` plus the `go.mod`
/// requirements in the module cache, `node_modules`, the gems `Gemfile.lock` names. `pip install -e` and vendored code inside
/// `root` are project files already, so `root` itself is never returned.
///
/// Nothing the project ships is run (#183). A toolchain runs from `/`, where no
/// `rust-toolchain.toml` names a `rustc` of the project's choosing and no `go.mod` asks for a Go
/// to download, and Go is held to the one installed as well.
///
/// ponytail: spawns the toolchain on every `d` that leaves the project. Cache per root if it
/// ever shows.
pub fn external_roots(kind: Kind, root: &Path) -> Vec<PathBuf> {
    let run = |cmd: &str, args: &[&str]| -> Option<String> {
        let out = std::process::Command::new(cmd)
            .args(args)
            .current_dir("/")
            .env("GOTOOLCHAIN", "local")
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let mut dirs: Vec<PathBuf> = match kind {
        // A `.venv` is read, not run: its `bin/python` is whatever the repository put there. The
        // packages are in `lib/pythonX.Y/site-packages`, and the standard library is beside the
        // interpreter the venv was made from, `pyvenv.cfg`'s `home`, once its symlinks are
        // followed out of Homebrew's `opt` or uv's short name.
        Kind::Python => {
            let venv = root.join(".venv");
            let lib = std::fs::read_dir(venv.join("lib"))
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.file_name())
                .find(|n| n.to_string_lossy().starts_with("python"));
            match lib {
                Some(lib) => {
                    let cfg = std::fs::read_to_string(venv.join("pyvenv.cfg")).unwrap_or_default();
                    let home = cfg.lines().find_map(|l| {
                        let (key, value) = l.split_once('=')?;
                        (key.trim() == "home").then(|| PathBuf::from(value.trim()))
                    });
                    let stdlib = home
                        .and_then(|h| h.join(&lib).canonicalize().ok())
                        .and_then(|exe| Some(exe.parent()?.parent()?.join("lib").join(&lib)));
                    let site = venv.join("lib").join(&lib).join("site-packages");
                    let system = cfg.lines().any(|l| {
                        l.split_once('=').is_some_and(|(k, v)| {
                            k.trim() == "include-system-site-packages"
                                && v.trim().eq_ignore_ascii_case("true")
                        })
                    });
                    let base = stdlib
                        .as_ref()
                        .filter(|_| system)
                        .map(|s| s.join("site-packages"));
                    stdlib.into_iter().chain([site]).chain(base).collect()
                }
                None => run(
                    "python3",
                    &["-c", "import sys; print('\\n'.join(sys.path))"],
                )
                .unwrap_or_default()
                .lines()
                .map(PathBuf::from)
                .collect(),
            }
        }
        Kind::Rust => {
            let mut dirs: Vec<PathBuf> = run("rustc", &["--print", "sysroot"])
                .map(|s| PathBuf::from(s.trim()).join("lib/rustlib/src/rust/library"))
                .into_iter()
                .collect();
            let cargo = std::env::var_os("CARGO_HOME")
                .map_or_else(|| home.join(".cargo"), PathBuf::from)
                .join("registry/src");
            // `[[package]]` tables: `name = "x"` then `version = "y"` is the directory `x-y`.
            let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap_or_default();
            let indexes: Vec<PathBuf> = std::fs::read_dir(&cargo)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .collect();
            let mut name = None;
            for line in lock.lines() {
                if let Some(n) = line.strip_prefix("name = ") {
                    name = Some(n.trim_matches('"').to_owned());
                } else if let (Some(v), Some(n)) = (line.strip_prefix("version = "), name.take()) {
                    let crate_dir = format!("{n}-{}", v.trim_matches('"'));
                    dirs.extend(indexes.iter().map(|index| index.join(&crate_dir)));
                }
            }
            dirs
        }
        Kind::Go => {
            let env = run("go", &["env", "GOROOT", "GOMODCACHE"]).unwrap_or_default();
            let mut env = env.lines();
            let mut dirs = Vec::new();
            if let Some(goroot) = env.next() {
                dirs.push(PathBuf::from(goroot).join("src"));
            }
            let cache = env.next().map(PathBuf::from).unwrap_or_default();
            // `require` lines, `path vX.Y.Z`; the cache escapes upper case as `!x`.
            let gomod = std::fs::read_to_string(root.join("go.mod")).unwrap_or_default();
            for line in gomod.lines() {
                let mut parts = line
                    .trim()
                    .trim_start_matches("require ")
                    .split_whitespace();
                if let (Some(path), Some(v)) = (parts.next(), parts.next())
                    && v.starts_with('v')
                {
                    let escaped: String = path
                        .chars()
                        .flat_map(|c| {
                            if c.is_ascii_uppercase() {
                                vec!['!', c.to_ascii_lowercase()]
                            } else {
                                vec![c]
                            }
                        })
                        .collect();
                    dirs.push(cache.join(format!("{escaped}@{v}")));
                }
            }
            dirs
        }
        Kind::TsJs => node_modules(root, root),
        // Zig's standard library, where its own `zig env` says it is. The dependencies of a
        // project live in the global package cache under hashed directory names no source line
        // spells out, so they are left out.
        Kind::Zig => zig_roots(&run("zig", &["env"]).unwrap_or_default()),
        // The system headers, which is where a C or C++ project's standard library and most of
        // its dependencies are: the SDK the toolchain reports on macOS, `/usr/include` on Linux,
        // and the two prefixes a package manager installs into. There is no per-project manifest
        // to read — what a build system was told with `-I` is not in the source — so the
        // directories are the same for every project, and the ones that do not exist fall out
        // below.
        //
        // Then Objective-C's, which only an Objective-C file reads ([`objc_root`]): the SDK's
        // frameworks, UIKit's under `iOSSupport`, those an umbrella framework holds (vImage in
        // Accelerate), and CocoaPods' `Pods/`, gitignored as `node_modules` is (#417).
        Kind::C => {
            let sdk = run("xcrun", &["--show-sdk-path"]).map(|s| PathBuf::from(s.trim()));
            let mut dirs = vec![PathBuf::from("/usr/include")];
            dirs.extend(sdk.iter().map(|sdk| sdk.join("usr/include")));
            dirs.push(PathBuf::from("/usr/local/include"));
            dirs.push(PathBuf::from("/opt/homebrew/include"));
            if let Some(sdk) = &sdk {
                let top = [
                    "System/Library/Frameworks",
                    "System/iOSSupport/System/Library/Frameworks",
                ]
                .map(|d| sdk.join(d));
                let umbrellas = (top.iter())
                    .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
                    .map(|e| e.path().join("Frameworks"))
                    .filter(|d| d.is_dir())
                    .collect::<Vec<_>>();
                dirs.extend(top);
                dirs.extend(umbrellas);
            }
            dirs.push(root.join("Pods"));
            dirs
        }
        // Where `protoc` installs the well-known types (`google/protobuf/timestamp.proto`), as
        // Homebrew and a Linux package lay it out. buf keeps a module's dependencies in a cache
        // under hashed directories no import spells, and is left out, as Zig's package cache is.
        Kind::Proto => [
            "/opt/homebrew/include",
            "/usr/local/include",
            "/usr/include",
        ]
        .map(PathBuf::from)
        .to_vec(),
        // Composer installs a project's dependencies into `vendor/`, as source, and gitignores
        // it, so the project walk does not list it: it is outside in the same way `node_modules`
        // is. PHP's own library is built into the interpreter and has no source to read.
        Kind::Php => vec![root.join("vendor")],
        // Where SwiftPM checks a package's dependencies out, as source. The standard library is
        // not there: the toolchain ships it compiled, with `.swiftinterface` stubs beside it and
        // no `.swift` file to read.
        Kind::Swift => vec![root.join(".build/checkouts")],
        // The gems `Gemfile.lock` names, the standard library and the core's signatures (#369).
        // The `ruby` on the PATH is asked once a session: it answers the same every time.
        Kind::Ruby => {
            static ASKED: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
            let env: Vec<PathBuf> = ["GEM_HOME", "GEM_PATH"]
                .into_iter()
                .filter_map(std::env::var_os)
                .flat_map(|v| std::env::split_paths(&v).collect::<Vec<_>>())
                .collect();
            let script = "puts RbConfig::CONFIG['rubylibdir'], Gem.path";
            ruby_roots(root, &home, &env, || {
                ASKED.get_or_init(|| run("ruby", &["-e", script])).clone()
            })
        }
        // The module directories of `PSModulePath` (#420). Only scripts and modules count: the
        // built-in cmdlets are compiled and have no source to find.
        Kind::PowerShell => {
            let pwsh = std::env::var_os("PATH").and_then(|p| {
                std::env::split_paths(&p)
                    .map(|d| d.join("pwsh"))
                    .find(|p| p.is_file())
            });
            powershell_roots(std::env::var_os("PSModulePath"), &home, pwsh)
        }
        // The packages `pub get` lists in `.dart_tool/package_config.json`, the pub cache's and
        // the Flutter SDK's, and the `lib/` of the SDK of the `dart` on the PATH (#414).
        Kind::Dart => dart_roots(root, dart_sdk()),
        Kind::Cmake => cmake_roots(std::env::var_os("PATH").and_then(|p| {
            std::env::split_paths(&p)
                .map(|d| d.join("cmake"))
                .find(|p| p.is_file())
        })),
        // Java and Kotlin have no roots yet: the JDK and Gradle caches are their own lookups.
        // C# has nothing to point at: a NuGet package is compiled
        // assemblies, and the runtime's own source is not on the machine at all. Lua has no root
        // to ask for either: `package.path` is whatever the interpreter embedding it was built
        // with, and a Neovim or a LuaRocks tree is not a standard library any project can be
        // assumed to use. Elixir's standard library ships compiled — an installed Elixir has
        // `.beam` files, not `.ex` — and its dependencies are the project's `deps/`, which
        // depend on the open file: [`mix_deps`]. `d` stays inside the project for all of them,
        // as for the rest.
        Kind::Jvm
        | Kind::CSharp
        | Kind::Lua
        | Kind::Nix
        | Kind::Starlark
        | Kind::Elixir
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml
        | Kind::Markdown
        | Kind::Graphql
        | Kind::Css
        | Kind::Html => Vec::new(),
    };
    // The order is deliberate, so no sort: `sys.path` can list a directory twice, far apart.
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| d.is_dir() && d != root && !d.as_os_str().is_empty() && seen.insert(d.clone()));
    dirs
}
/// The standard library directory in the output of `zig env`, which is JSON on some versions and
/// ZON on others: the value is the first quoted string after the key either way. A version that
/// reports no `std_dir` still reports the library directory it sits in. Empty when `zig` is not
/// on the PATH, as every root is when its toolchain is not installed.
pub(super) fn zig_roots(env: &str) -> Vec<PathBuf> {
    let value = |key: &str| {
        let rest = env.split_once(key)?.1.trim_start_matches('"');
        let rest = rest.split_once('"')?.1;
        Some(PathBuf::from(rest.split_once('"')?.0))
    };
    value("std_dir")
        .or_else(|| value("lib_dir").map(|d| d.join("std")))
        .into_iter()
        .collect()
}
/// The `node_modules` a TypeScript file of the project `root` resolves an import in: the one of
/// every directory from the file's up to `root`, nearest first, as Node walks them (#100). A
/// workspace keeps a package's dependencies beside the package, and pnpm keeps a dependency's own
/// under `.pnpm/<name>@<version>/node_modules`. A package installed in several is loaded from
/// one: [`package_copy`].
pub fn node_modules(root: &Path, file: &Path) -> Vec<PathBuf> {
    file.ancestors()
        .take_while(|dir| dir.starts_with(root))
        .map(|dir| dir.join("node_modules"))
        .filter(|dir| dir.is_dir())
        .collect()
}
/// The `deps/` a Mix project of the project `root` fetches its dependencies into, as source, for
/// an Elixir file in `file` (#437): beside every `mix.exs` from the file's directory up to `root`,
/// nearest first, so an umbrella app finds the umbrella's. `mix new` gitignores it, so the project
/// walk does not list it, as `node_modules` for TypeScript.
pub fn mix_deps(root: &Path, file: &Path) -> Vec<PathBuf> {
    file.ancestors()
        .take_while(|dir| dir.starts_with(root))
        .filter(|dir| dir.join("mix.exs").is_file())
        .map(|dir| dir.join("deps"))
        .filter(|dir| dir.is_dir())
        .collect()
}
/// `require("module").builtinModules` of Node 26, the bare names: no `_`, `/` or `node:` ones.
#[rustfmt::skip]
const NODE_BUILTINS: &[&str] = &[
    "assert", "async_hooks", "buffer", "child_process", "cluster", "console", "constants", "crypto",
    "dgram", "diagnostics_channel", "dns", "domain", "events", "fs", "http", "http2", "https",
    "inspector", "module", "net", "os", "path", "perf_hooks", "process", "punycode", "querystring",
    "readline", "repl", "stream", "string_decoder", "sys", "timers", "tls", "trace_events", "tty",
    "url", "util", "v8", "vm", "wasi", "worker_threads", "zlib",
];
/// The copy of the package a TypeScript import of `module` loads from a file with
/// [`node_modules`] `roots`, with its files among `files` (#141). Each root that has the package
/// (`lib`, `@scope/pkg`) or its types (`@types/lib`, `@types/scope__pkg`) holds a copy, whichever
/// of the two are there. The nearest is the one, unless it lacks the whole `module` and has no
/// `exports` map in its `package.json`: Node then goes on to the next `node_modules`, as it does
/// when `lib/extra` is not in the nearer one. A copy of JavaScript alone takes in the nearest
/// declaration files of the package further up, its own or its `@types`, which TypeScript reads;
/// TypeScript source counts as declarations. A link is followed, so
/// pnpm's `node_modules/lib` is the version of the store it points at, spelled under the root it
/// lies in, as the files walked from there are; one that leads out of the roots (`npm link`, a
/// pnpm store outside them) is a copy of no file. A copy whose `exports` map the path would come
/// from, lacking it as a file, is no copy to narrow to. A workspace package linked in is read from
/// the TypeScript and JavaScript among `own`, the files of the project at `root`, and `None` when
/// the copy chosen is one: no copy
/// outside is it. Empty when no root has the package (an ambient `declare module`, and a `node:`
/// module, which no package directory is called) and for a bare module of Node's own: `buffer`
/// is not the npm polyfill of that name but `@types/node`'s `declare module`, which TypeScript
/// takes over any `node_modules`.
pub fn package_copy(
    root: &Path,
    roots: &[PathBuf],
    files: &[PathBuf],
    own: &[PathBuf],
    module: &[String],
) -> Option<PackageCopy> {
    let (name, parts) = match module {
        [scope, pkg, ..] if scope.starts_with('@') => (format!("{scope}/{pkg}"), 2),
        [pkg, ..] if !NODE_BUILTINS.contains(&pkg.as_str()) => (pkg.clone(), 1),
        _ => return Some(PackageCopy::default()),
    };
    let types = format!("@types/{}", name.trim_start_matches('@').replace('/', "__"));
    // The project is a package too, its dependencies in a `node_modules` below it.
    let project = root.canonicalize().ok();
    let real: Vec<(&PathBuf, PathBuf)> = roots
        .iter()
        .filter_map(|r| Some((r, r.canonicalize().ok()?)))
        .collect();
    let spelled = |d: &Path| {
        real.iter()
            .find_map(|(r, real)| Some(r.join(d.strip_prefix(real).ok()?)))
    };
    let copy_in = |files: &[PathBuf], dirs: Vec<PathBuf>| PackageCopy {
        files: files
            .iter()
            .filter(|p| in_copy(p, &dirs))
            .cloned()
            .collect(),
        dirs,
        parts,
    };
    let mut chosen = None;
    for (i, level) in roots.iter().enumerate() {
        let dirs: Vec<PathBuf> = [&name, &types]
            .iter()
            .filter_map(|d| level.join(d).canonicalize().ok())
            .collect();
        if dirs.is_empty() {
            continue;
        }
        let linked = project
            .as_ref()
            .is_some_and(|p| dirs.iter().any(|d| in_copy(d, std::slice::from_ref(p))));
        let copy = match linked {
            // The project's own TypeScript and JavaScript: a readme says no path is there.
            true => {
                let own: Vec<PathBuf> = project
                    .iter()
                    .flat_map(|p| own.iter().map(|f| p.join(f)))
                    .filter(|f| kind_of(f) == Some(Kind::TsJs))
                    .collect();
                copy_in(&own, dirs)
            }
            false => copy_in(files, dirs.iter().filter_map(|d| spelled(d)).collect()),
        };
        let whole = copy.module(module).is_some_and(|(n, _)| n == module.len());
        let exports = std::fs::read_to_string(level.join(&name).join("package.json"))
            .is_ok_and(|text| exports_map(&text));
        // The map is what Node loads the path from, and it is not read here: as without a copy.
        if exports && !whole {
            return Some(PackageCopy::default());
        }
        let stop = whole || copy.files.is_empty();
        if chosen.is_none() || stop {
            chosen = Some((i, linked, copy));
        }
        if stop {
            break;
        }
    }
    let Some((i, linked, mut copy)) = chosen else {
        return Some(PackageCopy::default());
    };
    if linked {
        return None;
    }
    // What TypeScript reads: declarations, and TypeScript source a package may ship instead.
    let typed = |f: &PathBuf| {
        f.extension()
            .is_some_and(|e| ["ts", "tsx", "mts", "cts"].iter().any(|t| e == *t))
    };
    let declarations = |d: &PathBuf| -> Vec<PathBuf> {
        files
            .iter()
            .filter(|f| typed(f) && in_copy(f, std::slice::from_ref(d)))
            .cloned()
            .collect()
    };
    // A copy of JavaScript alone borrows the nearest declarations further up, a package's own
    // before its `@types`, as TypeScript reads them there; only those, not the JavaScript beside.
    if !copy.files.is_empty() && !copy.files.iter().any(typed) {
        let further = roots[i + 1..].iter().find_map(|r| {
            [&name, &types]
                .iter()
                .filter_map(|d| spelled(&r.join(d).canonicalize().ok()?))
                .map(|d| (declarations(&d), d))
                .find(|(found, _)| !found.is_empty())
        });
        if let Some((found, dir)) = further {
            copy.files.extend(found);
            copy.dirs.push(dir);
        }
    }
    Some(copy)
}
/// Whether a TypeScript import of `module` from the directory `dir` of the project `root`, whose
/// files are `files`, loads a package that is not installed (#392): a bare specifier (no
/// relative path, no `@/`, `~/` or `#` alias, no alias of the `tsconfig.json`, no module of
/// Node's own, none with a scheme such as `node:` or `virtual:`) that no workspace package of the
/// project is called (the `name` of a `package.json` among `files`), that no `node_modules`
/// from `dir` up holds, itself or its types (past the project's root too, as Node looks there),
/// and that no `declare module` of a `.d.ts` among `files` types, `*` patterns included.
pub fn package_missing(root: &Path, files: &[PathBuf], dir: &Path, module: &[String]) -> bool {
    static NAME: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r#""name"\s*:\s*"([^"]*)""#).unwrap());
    let name = match module {
        [scope, pkg, ..] if scope.len() > 1 && scope.starts_with('@') => format!("{scope}/{pkg}"),
        [pkg, ..]
            if !pkg.starts_with(['.', '@', '~', '#'])
                && !pkg.contains(':')
                && !NODE_BUILTINS.contains(&pkg.as_str()) =>
        {
            pkg.clone()
        }
        _ => return false,
    };
    if ts_alias(root, files, dir, &module.join("/")) {
        return false;
    }
    let types = format!("@types/{}", name.trim_start_matches('@').replace('/', "__"));
    let installed = root.join(dir).ancestors().any(|d| {
        [&name, &types]
            .iter()
            .any(|p| d.join("node_modules").join(p).exists())
    });
    let workspace = || {
        files
            .iter()
            .filter(|f| f.file_name().is_some_and(|n| n == "package.json"))
            .filter_map(|f| std::fs::read_to_string(root.join(f)).ok())
            .any(|t| NAME.captures(&t).is_some_and(|c| c[1] == name))
    };
    // ponytail: reads every `.d.ts` of the project, only for a package nothing else supplies.
    let declared = || {
        static DECLARE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r#"declare\s+module\s+['"]([^'"]+)['"]"#).unwrap()
        });
        let spec = module.join("/");
        files
            .iter()
            .filter(|f| f.to_string_lossy().ends_with(".d.ts"))
            .filter_map(|f| std::fs::read_to_string(root.join(f)).ok())
            .any(|t| {
                DECLARE
                    .captures_iter(&t)
                    .any(|c| match c[1].split_once('*') {
                        Some((pre, post)) => {
                            spec.len() >= pre.len() + post.len()
                                && spec.starts_with(pre)
                                && spec.ends_with(post)
                        }
                        None => c[1] == spec,
                    })
            })
    };
    !installed && !workspace() && !declared()
}
/// Whether the `package.json` `text` has an `exports` map, a top-level key that is not `null`.
/// Strings are read whole, so neither a nested `exports` nor a brace inside one counts.
fn exports_map(text: &str) -> bool {
    let bytes = text.as_bytes();
    let (mut i, mut depth) = (0, 0);
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'"' => {
                let start = i + 1;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                let key = text.get(start..i).unwrap_or_default();
                let value = text.get(i + 1..).unwrap_or_default().trim_start();
                if depth == 1
                    && key == "exports"
                    && let Some(value) = value.strip_prefix(':')
                {
                    return !value.trim_start().starts_with("null");
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}
/// A copy of an npm package [`package_copy`] finds: the directories it is in (the package's and
/// its types'), its walked files there, and how many parts of a module path its name is.
#[derive(Debug, Default, PartialEq)]
pub struct PackageCopy {
    pub dirs: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
    pub parts: usize,
}
impl PackageCopy {
    /// The files an import of the package itself loads, in each of its directories: those its
    /// `package.json` names in `types` or `typings`, else in `main` or `module` with the `.d.ts`
    /// beside a JavaScript one, else its `index` files. Read as `ts_aliases` reads a key.
    pub fn entries(&self) -> Vec<PathBuf> {
        static KEY: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r#""(types|typings|main|module)"\s*:\s*"([^"]*)""#).unwrap()
        });
        let mut entries = Vec::new();
        for dir in &self.dirs {
            let text = std::fs::read_to_string(dir.join("package.json")).unwrap_or_default();
            let named = |keys: &[&str]| -> Vec<PathBuf> {
                let mut found = Vec::new();
                for c in KEY.captures_iter(&text).filter(|c| keys.contains(&&c[1])) {
                    let entry = c[2].trim_start_matches("./");
                    let stem = [".js", ".mjs", ".cjs"]
                        .iter()
                        .find_map(|e| entry.strip_suffix(e))
                        .unwrap_or(entry);
                    for f in [entry.to_owned(), format!("{stem}.d.ts")] {
                        found.extend(self.files.iter().filter(|p| **p == dir.join(&f)).cloned());
                    }
                }
                found
            };
            let index = || -> Vec<PathBuf> {
                let at_root = |f: &&PathBuf| f.parent() == Some(dir.as_path());
                let index = |f: &&PathBuf| {
                    f.file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with("index."))
                };
                self.files
                    .iter()
                    .filter(at_root)
                    .filter(index)
                    .cloned()
                    .collect()
            };
            let found = [&["types", "typings"][..], &["main", "module"]]
                .iter()
                .map(|keys| named(keys))
                .find(|found| !found.is_empty());
            entries.extend(found.unwrap_or_else(index));
        }
        entries
    }
    /// The files of the copy `module` names, with how many of its parts that is. The package's
    /// own parts are the copy, whatever its directory is called (pnpm's alias `cookie` links a
    /// store directory called `cookie-es`); the rest of the path is matched below the copy's
    /// directories and shortened from its end as [`module_among`] shortens it, down to the whole
    /// copy. `None` for a copy of no files.
    pub fn module(&self, module: &[String]) -> Option<(usize, Vec<PathBuf>)> {
        if self.files.is_empty() {
            return None;
        }
        let below: Vec<Below> = self
            .files
            .iter()
            .map(|file| Below {
                path: self
                    .dirs
                    .iter()
                    .find_map(|d| file.strip_prefix(d).ok())
                    .unwrap_or(file),
                file,
            })
            .collect();
        let rest = module.get(self.parts..).unwrap_or_default();
        Some(match module_among(&below, rest, None) {
            Some((n, found)) => (
                self.parts + n,
                found.iter().map(|b| b.file.clone()).collect(),
            ),
            None => (self.parts, self.files.clone()),
        })
    }
}
/// A file of a copy, matched by its path below the copy's directory.
#[derive(Clone)]
struct Below<'a> {
    path: &'a Path,
    file: &'a PathBuf,
}
impl AsRef<Path> for Below<'_> {
    fn as_ref(&self) -> &Path {
        self.path
    }
}
/// Whether `path` is a file of the package in the directories `copy`, and not of a package it
/// depends on, in a `node_modules` of its own.
pub fn in_copy(path: &Path, copy: &[PathBuf]) -> bool {
    copy.iter().any(|dir| {
        path.strip_prefix(dir)
            .is_ok_and(|rest| !rest.components().any(|c| c.as_os_str() == "node_modules"))
    })
}
/// Every file of `kind` under `dirs`, as absolute paths. Nothing is ignored: `node_modules`
/// and `site-packages` are gitignored by design and are exactly what is wanted here. What no Go
/// import can reach is left out: a `_test.go` file, a `testdata` directory, and a nested module
/// (GOROOT's `cmd`, the toolchain's own source), which is a root of its own when it is a
/// dependency.
pub fn external_files(kind: Kind, dirs: &[PathBuf]) -> Vec<PathBuf> {
    let go = kind == Kind::Go;
    let python = kind == Kind::Python;
    // A link is spelled through the roots a C file reads, never the frameworks: `usr/include`'s
    // `tcl.h` links into `Tcl.framework`, and stays `tcl.h` (#417).
    let spelled: Vec<PathBuf> = (dirs.iter())
        .filter(|d| !objc_frameworks(d))
        .cloned()
        .collect();
    let real = real_dirs(&spelled);
    let mut files = Vec::new();
    for dir in dirs {
        // Homebrew's Rust ships the sysroot `library` with a copy of itself inside; every
        // definition would come up twice.
        let copy = dir.file_name().map(std::ffi::OsStr::to_owned);
        let others: Vec<PathBuf> = dirs.iter().filter(|o| *o != dir).cloned().collect();
        // Of an SDK's frameworks, each one's `Headers`, a link into `Versions/Current` that is
        // walked through, so a header is read once (#417).
        let frameworks = kind == Kind::C && objc_frameworks(dir);
        let walk = ignore::WalkBuilder::new(dir)
            .filter_entry(move |e| {
                let is_dir = e.depth() > 0 && e.file_type().is_some_and(|t| t.is_dir());
                let unreachable = go
                    && is_dir
                    && (e.file_name() == "testdata" || e.path().join("go.mod").is_file());
                let base_packages = python
                    && is_dir
                    && e.depth() == 1
                    && (e.file_name() == "site-packages" || e.file_name() == "dist-packages");
                let nested = is_dir && others.iter().any(|o| o == e.path());
                let header = match e.depth() {
                    1 => e.file_name().to_string_lossy().ends_with(".framework"),
                    2 => e.file_name() == "Headers",
                    _ => true,
                };
                !unreachable
                    && !base_packages
                    && !nested
                    && (!frameworks || header)
                    && (e.depth() != 1 || Some(e.file_name()) != copy.as_deref())
            })
            .follow_links(kind == Kind::Proto || kind == Kind::Cmake || frameworks)
            .hidden(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .ignore(false)
            .parents(false)
            .build();
        files.extend(
            walk.filter_map(Result::ok)
                .map(|e| match kind == Kind::C && e.path_is_symlink() {
                    true => c_spelled(e.path(), &real),
                    false => e.into_path(),
                })
                .filter(|p| p.is_file() && (kind_of(p) == Some(kind) || core_signature(kind, p)))
                .filter(|p| !(go && p.to_string_lossy().ends_with("_test.go"))),
        );
    }
    // A header linked under another name is one file, listed once (#382).
    if kind == Kind::C {
        let mut seen = std::collections::HashSet::new();
        files.retain(|f| seen.insert(f.clone()));
    }
    files
}
/// The C++ standard library directories under `roots`, `c++/v1` and `c++/<version>`, which a
/// compiler searches for an `#include <…>` of a C++ file before the roots themselves.
pub fn cpp_dirs(roots: &[PathBuf]) -> Vec<PathBuf> {
    roots
        .iter()
        .flat_map(|r| {
            std::fs::read_dir(r.join("c++"))
                .into_iter()
                .flatten()
                .flatten()
        })
        .map(|e| e.path())
        .filter(|d| d.is_dir())
        .collect()
}
/// `path`, a file under one of `dirs`, as the walk of `dirs` spells it: through its links, under
/// the directory its target lies in when that is one of `dirs`, so a header linked under a second
/// name (`pthread.h` to `pthread/pthread.h`) is that header (#382). A link out of them keeps its
/// own name: Homebrew's `include/` is links into its kegs.
/// `dirs` are [`real_dirs`].
pub fn c_spelled(path: &Path, dirs: &[(PathBuf, PathBuf)]) -> PathBuf {
    let Ok(real) = path.canonicalize() else {
        return path.to_path_buf();
    };
    dirs.iter()
        .find_map(|(d, r)| Some(d.join(real.strip_prefix(r).ok()?)))
        .unwrap_or_else(|| path.to_path_buf())
}
/// Each of `dirs` with the directory it is once its links are followed.
pub fn real_dirs(dirs: &[PathBuf]) -> Vec<(PathBuf, PathBuf)> {
    (dirs.iter())
        .filter_map(|d| Some((d.clone(), d.canonicalize().ok()?)))
        .collect()
}
/// Whether `path` is one of the RBS signatures of Ruby's core, `core/*.rbs` of the `rbs` gem
/// (#369): the classes written in C have no other source. Elsewhere a `.rbs` repeats a `.rb`
/// beside it, so it is no file of any kind, the project's own `sig/` included.
fn core_signature(kind: Kind, path: &Path) -> bool {
    kind == Kind::Ruby
        && path.extension().is_some_and(|e| e == "rbs")
        && path.ancestors().any(|a| {
            a.ends_with("core")
                && a.parent()
                    .and_then(Path::file_name)
                    .is_some_and(|n| n.to_string_lossy().starts_with("rbs-"))
        })
}
/// The files a reader is shown last: tests, mocks, fixtures, generated code and vendored copies
/// (#81). A row ending in `/` is a directory anywhere in the path; the rest match the file name,
/// where a `*` at either end stands for any run of characters. One table, so `u` and the
/// candidates of `d` agree on what a test file is.
const LAST: &[&str] = &[
    "test/",
    "tests/",
    "__tests__/",
    "spec/",
    "specs/",
    "testdata/",
    "fixtures/",
    "__fixtures__/",
    "mocks/",
    "__mocks__/",
    "vendor/",
    "third_party/",
    // A test file, by the naming every language settled on: `test_user.py`, `user_test.go`,
    // `user_spec.rb`, `user.test.ts`, `user.spec.ts`.
    "test_*",
    "conftest.py",
    "*_test.*",
    "*_spec.*",
    "*.test.*",
    "*.spec.*",
    // Generated: protobuf, and the `.gen.`/`.generated.` convention the code generators use.
    "*_pb2.py",
    "*_pb2_grpc.py",
    "*.pb.go",
    "*.gen.go",
    "*.generated.*",
];
/// Where a hit sorts in a result list: what the reader came for first (#81).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// A line that declares the word, by the [`def_patterns`] of the file's kind.
    Declaration,
    /// The file on screen.
    Open,
    /// The rest of the project's code.
    Code,
    /// A file of [`LAST`].
    Tests,
}
/// How a hit in `path` sorts, with the open file at `here` and `declaration` telling whether the
/// line declares the word: its [`Tier`], then how many directories away from the open file it
/// lives, the nearest first. The caller breaks a tie by path and line.
///
/// `u` ranks its hits with this, and `d` demotes the candidates in [`Tier::Tests`] with it, so
/// one table decides for both. The open file is never demoted: it is what the reader is reading.
pub fn rank(path: &Path, here: Option<&Path>, declaration: bool) -> (Tier, usize) {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let last = LAST.iter().any(|row| match row.strip_suffix('/') {
        Some(dir) => path
            .parent()
            .is_some_and(|p| p.iter().any(|c| c == std::ffi::OsStr::new(dir))),
        None => match (row.strip_prefix('*'), row.strip_suffix('*')) {
            (Some(_), Some(_)) => name.contains(row.trim_matches('*')),
            (Some(suffix), None) => name.ends_with(suffix),
            (None, Some(prefix)) => name.starts_with(prefix),
            (None, None) => name == *row,
        },
    });
    let tier = if last && here != Some(path) {
        Tier::Tests
    } else if declaration {
        Tier::Declaration
    } else if here == Some(path) {
        Tier::Open
    } else {
        Tier::Code
    };
    let dirs = |p: &Path| p.parent().map_or(0, |d| d.components().count());
    let steps = here.map_or(0, |h| {
        let shared = path
            .parent()
            .unwrap_or(Path::new(""))
            .components()
            .zip(h.parent().unwrap_or(Path::new("")).components())
            .take_while(|(a, b)| a == b)
            .count();
        dirs(path) + dirs(h) - 2 * shared
    });
    (tier, steps)
}
/// The `GOOS` and `GOARCH` of the machine merl runs on, which is what `go build` targets there.
pub fn go_host() -> (&'static str, &'static str) {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        os => os,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "x86" => "386",
        arch => arch,
    };
    (os, arch)
}
/// What decides whether `go build` compiles a file: the platform, cgo, and the tags of `-tags`.
pub struct GoBuild {
    pub os: &'static str,
    pub arch: &'static str,
    pub cgo: bool,
    pub tags: Vec<String>,
}

impl GoBuild {
    /// A plain `go build` on this machine: its platform, cgo on, no tag set.
    pub fn host() -> Self {
        let (os, arch) = go_host();
        Self {
            os,
            arch,
            cgo: true,
            tags: Vec::new(),
        }
    }

    /// With what the environment adds, the way `go build` and gopls read it: `CGO_ENABLED=0`,
    /// and the `-tags=a,b` of `GOFLAGS`, where the last one counts.
    // ponytail: the environment only. `go env -w` and a missing C compiler (cgo off) are not
    // seen; ask `go env` once at startup if that ever misleads.
    pub fn env(mut self, cgo_enabled: Option<&str>, goflags: Option<&str>) -> Self {
        self.cgo = cgo_enabled != Some("0");
        let flags = goflags.unwrap_or("").split_whitespace();
        if let Some(tags) = flags
            .filter_map(|f| f.trim_start_matches('-').strip_prefix("tags="))
            .next_back()
        {
            self.tags = tags.split(',').map(str::to_owned).collect();
        }
        self
    }
}

pub(super) const GO_UNIX: &[&str] = &[
    "aix",
    "android",
    "darwin",
    "dragonfly",
    "freebsd",
    "hurd",
    "illumos",
    "ios",
    "linux",
    "netbsd",
    "openbsd",
    "solaris",
];
pub(super) const GO_OS: &[&str] = &["js", "nacl", "plan9", "wasip1", "windows", "zos"];
pub(super) const GO_ARCH: &[&str] = &[
    "386",
    "amd64",
    "amd64p32",
    "arm",
    "arm64",
    "arm64be",
    "armbe",
    "loong64",
    "mips",
    "mips64",
    "mips64le",
    "mips64p32",
    "mips64p32le",
    "mipsle",
    "ppc",
    "ppc64",
    "ppc64le",
    "riscv",
    "riscv64",
    "s390",
    "s390x",
    "sparc",
    "sparc64",
    "wasm",
];
/// Whether `build` compiles the Go file `path` with the text `text`: the `_GOOS`, `_GOARCH` and
/// `_GOOS_GOARCH` endings of its name and its `//go:build` line, read as `go build` reads them. A
/// tag is set when it is the platform, `cgo`, the compiler (`gc`), a release (`go1.21`: the
/// toolchain that builds the project has it) or one of `-tags`; any other (`gogit`, `ignore`) is
/// unset. `None` for a constraint that is not read: the `// +build` of before Go 1.17, a line
/// that does not parse.
pub fn go_built(path: &Path, text: &str, build: &GoBuild) -> Option<bool> {
    let is_os = |t: &str| GO_UNIX.contains(&t) || GO_OS.contains(&t);
    let tag = |t: &str| -> bool {
        match t {
            "unix" => GO_UNIX.contains(&build.os),
            t if is_os(t) => t == build.os,
            t if GO_ARCH.contains(&t) => t == build.arch,
            "cgo" => build.cgo,
            "gc" => true,
            t if t.starts_with("go1.") => true,
            t => build.tags.iter().any(|set| set == t),
        }
    };
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let stem = stem.strip_suffix("_test").unwrap_or(stem);
    let mut parts: Vec<&str> = stem.split('_').skip(1).collect();
    let mut named = true;
    if let Some(arch) = parts.pop_if(|p| GO_ARCH.contains(p)) {
        named &= arch == build.arch;
    }
    if let Some(os) = parts.pop_if(|p| is_os(p)) {
        named &= os == build.os;
    }
    if !named {
        return Some(false);
    }
    // A line inside a `/* */` block in front of the package clause is a comment's.
    let literal = literal_lines(Kind::Go, text);
    let head = || {
        let lines = text.lines().take_while(|l| !l.starts_with("package "));
        lines
            .enumerate()
            .filter(|(i, _)| !literal[*i])
            .map(|(_, l)| l)
    };
    let Some(expr) = head().find_map(|l| l.strip_prefix("//go:build ")) else {
        // The constraint of before Go 1.17 is not read: undecided, never "no constraint".
        return (!head().any(|l| l.starts_with("// +build"))).then_some(true);
    };
    // `!` binds tightest, then `&&`, then `||`.
    static TOKEN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"&&|\|\||[!()]|[\w.]+").unwrap());
    let tokens: Vec<&str> = TOKEN.find_iter(expr).map(|m| m.as_str()).collect();
    type Tag<'a> = &'a dyn Fn(&str) -> bool;
    fn any(t: &[&str], i: &mut usize, tag: Tag) -> Option<bool> {
        let mut value = all(t, i, tag)?;
        while t.get(*i) == Some(&"||") {
            *i += 1;
            value |= all(t, i, tag)?;
        }
        Some(value)
    }
    fn all(t: &[&str], i: &mut usize, tag: Tag) -> Option<bool> {
        let mut value = one(t, i, tag)?;
        while t.get(*i) == Some(&"&&") {
            *i += 1;
            value &= one(t, i, tag)?;
        }
        Some(value)
    }
    fn one(t: &[&str], i: &mut usize, tag: Tag) -> Option<bool> {
        let token = *t.get(*i)?;
        *i += 1;
        match token {
            "!" => one(t, i, tag).map(|v| !v),
            "(" => {
                let value = any(t, i, tag);
                *i += 1;
                value
            }
            "&&" | "||" | ")" => None,
            name => Some(tag(name)),
        }
    }
    any(&tokens, &mut 0, &tag)
}
