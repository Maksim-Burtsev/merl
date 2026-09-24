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
        Kind::Docker | Kind::Yaml => path == here,
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
        | Kind::Shell
        | Kind::Sql
        | Kind::Make => kind_of(path) == Some(kind),
    }
}
/// Where the standard library and the dependencies of the project at `root` live on this
/// machine, for a file of `kind`; empty when the toolchain is not installed. Each is asked the
/// way it answers itself: the project's `.venv`, or `sys.path` of the `python3` on the PATH,
/// `rustc --print sysroot` plus the registry crates `Cargo.lock` names, `GOROOT` plus the `go.mod`
/// requirements in the module cache, `node_modules`. `pip install -e` and vendored code inside
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
                    stdlib.into_iter().chain([site]).collect()
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
        Kind::C => {
            let mut dirs = vec![PathBuf::from("/usr/include")];
            if let Some(sdk) = run("xcrun", &["--show-sdk-path"]) {
                dirs.push(PathBuf::from(sdk.trim()).join("usr/include"));
            }
            dirs.push(PathBuf::from("/usr/local/include"));
            dirs.push(PathBuf::from("/opt/homebrew/include"));
            dirs
        }
        // Composer installs a project's dependencies into `vendor/`, as source, and gitignores
        // it, so the project walk does not list it: it is outside in the same way `node_modules`
        // is. PHP's own library is built into the interpreter and has no source to read.
        Kind::Php => vec![root.join("vendor")],
        // Where SwiftPM checks a package's dependencies out, as source. The standard library is
        // not there: the toolchain ships it compiled, with `.swiftinterface` stubs beside it and
        // no `.swift` file to read.
        Kind::Swift => vec![root.join(".build/checkouts")],
        // Java, Kotlin and Ruby have no roots yet: the JDK and Gradle caches, and a gem path,
        // are their own lookups. C# has nothing to point at: a NuGet package is compiled
        // assemblies, and the runtime's own source is not on the machine at all. Lua has no root
        // to ask for either: `package.path` is whatever the interpreter embedding it was built
        // with, and a Neovim or a LuaRocks tree is not a standard library any project can be
        // assumed to use. Elixir needs none: `mix` puts both the dependencies and their sources
        // in `deps/` inside the project, so they are project files already, and the standard
        // library ships compiled — an installed Elixir has `.beam` files, not `.ex`. `d` stays
        // inside the project for all of them, as for the rest.
        Kind::Jvm
        | Kind::Ruby
        | Kind::CSharp
        | Kind::Lua
        | Kind::Elixir
        | Kind::Shell
        | Kind::Sql
        | Kind::Make
        | Kind::Terraform
        | Kind::Docker
        | Kind::Yaml => Vec::new(),
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
/// `require("module").builtinModules` of Node 26, the bare names: no `_`, `/` or `node:` ones.
#[rustfmt::skip]
const NODE_BUILTINS: &[&str] = &[
    "assert", "async_hooks", "buffer", "child_process", "cluster", "console", "constants", "crypto",
    "dgram", "diagnostics_channel", "dns", "domain", "events", "fs", "http", "http2", "https",
    "inspector", "module", "net", "os", "path", "perf_hooks", "process", "punycode", "querystring",
    "readline", "repl", "stream", "string_decoder", "sys", "timers", "tls", "trace_events", "tty",
    "url", "util", "v8", "vm", "wasi", "worker_threads", "zlib",
];
/// The files among `files` of the copy of the package a TypeScript import of `module` loads
/// from a file with [`node_modules`] `roots` (#141). Each root that has the package (`lib`,
/// `@scope/pkg`) or its types (`@types/lib`, `@types/scope__pkg`) holds a copy: whichever of the
/// two are there, and when that is a package of JavaScript alone, the nearest `@types` of it
/// further up, which TypeScript reads for it. The nearest copy whose files have the whole
/// `module` is the one, as Node goes on to the next `node_modules` when `lib/extra` is not in
/// the nearer one; when none has it, the nearest. A link is followed, so pnpm's
/// `node_modules/lib` is the version of the store it points at, spelled under the root it lies
/// in, as the files walked from there are; one that leads out of the roots (a pnpm store outside
/// them) has no file there. Empty when no root has the package (an ambient `declare module`, and
/// a `node:` module, which no package directory is called) and for a bare module of Node's own:
/// `buffer` is not the npm polyfill of that name but `@types/node`'s `declare module`, which
/// TypeScript takes over any `node_modules`. `None` when a copy on the way is the source of the
/// project at `root`, a workspace package linked into `node_modules`: no copy outside is it.
pub fn package_copy(
    root: &Path,
    roots: &[PathBuf],
    files: &[PathBuf],
    module: &[String],
) -> Option<Vec<PathBuf>> {
    let name = match module {
        [scope, pkg, ..] if scope.starts_with('@') => format!("{scope}/{pkg}"),
        [pkg, ..] if !NODE_BUILTINS.contains(&pkg.as_str()) => pkg.clone(),
        _ => return Some(Vec::new()),
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
    let files_in = |dirs: &[PathBuf]| -> Vec<PathBuf> {
        files.iter().filter(|p| in_copy(p, dirs)).cloned().collect()
    };
    let mut nearest = None;
    for (i, level) in roots.iter().enumerate() {
        let dirs: Vec<PathBuf> = [&name, &types]
            .iter()
            .filter_map(|d| level.join(d).canonicalize().ok())
            .collect();
        if dirs.iter().any(|d| {
            project
                .as_ref()
                .is_some_and(|p| in_copy(d, std::slice::from_ref(p)))
        }) {
            return None;
        }
        let mut dirs: Vec<PathBuf> = dirs.iter().filter_map(|d| spelled(d)).collect();
        if dirs.is_empty() {
            continue;
        }
        let mut copy = files_in(&dirs);
        if !copy.iter().any(|f| declaration_file(f)) {
            let further = roots[i + 1..]
                .iter()
                .find_map(|r| spelled(&r.join(&types).canonicalize().ok()?));
            if let Some(further) = further {
                dirs.push(further);
                copy = files_in(&dirs);
            }
        }
        if module_among(&copy, module, None).is_some_and(|(n, _)| n == module.len()) {
            return Some(copy);
        }
        nearest.get_or_insert(copy);
    }
    Some(nearest.unwrap_or_default())
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
    let mut files = Vec::new();
    for dir in dirs {
        // Homebrew's Rust ships the sysroot `library` with a copy of itself inside; every
        // definition would come up twice.
        let copy = dir.file_name().map(std::ffi::OsStr::to_owned);
        let walk = ignore::WalkBuilder::new(dir)
            .filter_entry(move |e| {
                let unreachable = go
                    && e.depth() > 0
                    && e.file_type().is_some_and(|t| t.is_dir())
                    && (e.file_name() == "testdata" || e.path().join("go.mod").is_file());
                !unreachable && (e.depth() != 1 || Some(e.file_name()) != copy.as_deref())
            })
            .hidden(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .ignore(false)
            .parents(false)
            .build();
        files.extend(
            walk.filter_map(Result::ok)
                .map(ignore::DirEntry::into_path)
                .filter(|p| p.is_file() && kind_of(p) == Some(kind))
                .filter(|p| !(go && p.to_string_lossy().ends_with("_test.go"))),
        );
    }
    files
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
