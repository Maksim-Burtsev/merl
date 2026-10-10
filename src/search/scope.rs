use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

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
        | Kind::Zig
        | Kind::Proto
        | Kind::Shell
        | Kind::PowerShell
        | Kind::Dart
        | Kind::Cmake
        | Kind::Nix
        | Kind::Haskell
        | Kind::Ocaml
        | Kind::Fsharp
        | Kind::Julia
        | Kind::R
        | Kind::Perl
        | Kind::Gdscript
        | Kind::Solidity
        | Kind::Clojure
        | Kind::EmacsLisp
        | Kind::Scheme
        | Kind::CommonLisp
        | Kind::Starlark
        | Kind::Sql
        | Kind::Make
        | Kind::Graphql
        | Kind::Css => kind_of(path) == Some(kind),
        Kind::Elixir => kind_of(path) == Some(kind) && erlang(path) == erlang(here),
    }
}
pub fn external_roots(kind: Kind, root: &Path) -> Vec<PathBuf> {
    let run = |cmd: &str, args: &[&str]| -> Option<String> {
        let out = toolchain(cmd, args).output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let dirs: Vec<PathBuf> = match kind {
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
            let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap_or_default();
            dirs.extend(locked_crates(&cargo, &lock));
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
            let gomod = std::fs::read_to_string(root.join("go.mod")).unwrap_or_default();
            dirs.extend(required_modules(&cache, &gomod));
            dirs
        }
        Kind::TsJs => node_modules(root, root),
        Kind::Zig => zig_roots(&run("zig", &["env"]).unwrap_or_default()),
        Kind::C => c_roots(
            run("xcrun", &["--show-sdk-path"]).map(|s| PathBuf::from(s.trim())),
            root,
        ),
        Kind::Proto => [
            "/opt/homebrew/include",
            "/usr/local/include",
            "/usr/include",
        ]
        .map(PathBuf::from)
        .to_vec(),
        Kind::Php => vec![root.join("vendor")],
        Kind::Swift => vec![root.join(".build/checkouts")],
        Kind::Ruby => {
            static RUBY_ANSWER_THIS_SESSION: std::sync::OnceLock<Option<String>> =
                std::sync::OnceLock::new();
            let env: Vec<PathBuf> = ["GEM_HOME", "GEM_PATH"]
                .into_iter()
                .filter_map(std::env::var_os)
                .flat_map(|v| std::env::split_paths(&v).collect::<Vec<_>>())
                .collect();
            let script = "puts RbConfig::CONFIG['rubylibdir'], Gem.path";
            ruby_roots(root, &home, &env, || {
                RUBY_ANSWER_THIS_SESSION
                    .get_or_init(|| run("ruby", &["-e", script]))
                    .clone()
            })
        }
        Kind::PowerShell => {
            let pwsh = std::env::var_os("PATH").and_then(|p| {
                std::env::split_paths(&p)
                    .map(|d| d.join("pwsh"))
                    .find(|p| p.is_file())
            });
            powershell_roots(std::env::var_os("PSModulePath"), &home, pwsh)
        }
        Kind::Dart => dart_roots(root, dart_sdk()),
        Kind::Elixir => otp_roots(run(
            "erl",
            &[
                "-noshell",
                "-eval",
                r#"io:format("~s", [code:root_dir()]), halt()."#,
            ],
        )),
        Kind::Ocaml => ocaml_roots(run("ocamlc", &["-where"])),
        Kind::Cmake => cmake_roots(std::env::var_os("PATH").and_then(|p| {
            std::env::split_paths(&p)
                .map(|d| d.join("cmake"))
                .find(|p| p.is_file())
        })),
        Kind::Julia => {
            static SHARE: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
            let share = SHARE.get_or_init(|| {
                julia_share(&run(
                    "julia",
                    &["--startup-file=no", "-e", "print(Sys.BINDIR)"],
                )?)
            });
            let depot = julia_depot(std::env::var_os("JULIA_DEPOT_PATH"), &home);
            julia_roots(root, share.clone(), &depot)
        }
        Kind::Perl => {
            let inc = run("perl", &["-e", "print join qq{\\n}, @INC"]).unwrap_or_default();
            perl_roots(root, &inc)
        }
        Kind::EmacsLisp => vec![home.join(".emacs.d/elpa"), home.join(".config/emacs/elpa")],
        Kind::CommonLisp => vec![
            home.join("quicklisp/dists/quicklisp/software"),
            home.join("quicklisp/local-projects"),
        ],
        Kind::Scheme => {
            let script = "(for-each displayln (current-library-collection-paths))";
            (run("racket", &["-e", script]).unwrap_or_default().lines())
                .map(PathBuf::from)
                .collect()
        }
        Kind::Jvm
        | Kind::CSharp
        | Kind::Lua
        | Kind::Nix
        | Kind::Haskell
        | Kind::Fsharp
        | Kind::R
        | Kind::Gdscript
        | Kind::Solidity
        | Kind::Clojure
        | Kind::Starlark
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
    existing_once(dirs, root)
}
pub(super) fn toolchain(cmd: &str, args: &[&str]) -> std::process::Command {
    let mut command = std::process::Command::new(cmd);
    command
        .args(args)
        .current_dir("/")
        .env("GOTOOLCHAIN", "local");
    command
}
pub(super) fn existing_once(mut dirs: Vec<PathBuf>, root: &Path) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| d.is_dir() && d != root && !d.as_os_str().is_empty() && seen.insert(d.clone()));
    dirs
}
pub(super) fn locked_crates(registry: &Path, lock: &str) -> Vec<PathBuf> {
    let indexes: Vec<PathBuf> = std::fs::read_dir(registry)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    let mut dirs = Vec::new();
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
pub(super) fn required_modules(cache: &Path, gomod: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
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
pub(super) fn c_roots(sdk: Option<PathBuf>, root: &Path) -> Vec<PathBuf> {
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
pub fn node_modules(root: &Path, file: &Path) -> Vec<PathBuf> {
    file.ancestors()
        .take_while(|dir| dir.starts_with(root))
        .map(|dir| dir.join("node_modules"))
        .filter(|dir| dir.is_dir())
        .collect()
}
pub fn drop_farther_copies(roots: &[PathBuf], hits: Vec<Hit>) -> Vec<Hit> {
    let real: Vec<PathBuf> = roots.iter().filter_map(|r| r.canonicalize().ok()).collect();
    let rows: std::collections::HashSet<(PathBuf, String)> = (hits.iter())
        .filter_map(|h| Some((h.path.canonicalize().ok()?, h.text.trim().to_owned())))
        .collect();
    let mut loaded: std::collections::HashMap<PathBuf, Option<PathBuf>> =
        std::collections::HashMap::new();
    (hits.into_iter())
        .filter(|h| {
            !nested_packages(roots, &h.path)
                .into_iter()
                .any(|(dir, name)| {
                    let nearest = loaded.entry(dir.clone()).or_insert_with(|| {
                        let n = (roots.iter()).find_map(|r| r.join(&name).canonicalize().ok())?;
                        let other = dir.canonicalize().ok() != Some(n.clone());
                        (other && real.iter().any(|r| n.starts_with(r))).then_some(n)
                    });
                    nearest.as_ref().is_some_and(|n| {
                        let row = |rel: &Path| (n.join(rel), h.text.trim().to_owned());
                        (h.path.strip_prefix(&dir)).is_ok_and(|rel| rows.contains(&row(rel)))
                    })
                })
        })
        .collect()
}
fn nested_packages(roots: &[PathBuf], file: &Path) -> Vec<(PathBuf, PathBuf)> {
    let Some(root) = roots.iter().find(|r| file.starts_with(r)) else {
        return Vec::new();
    };
    let parts: Vec<_> = file
        .strip_prefix(root)
        .unwrap_or(file)
        .components()
        .collect();
    let mut dir = root.clone();
    let mut found = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        dir.push(part);
        if part.as_os_str() != "node_modules" || roots.contains(&dir) {
            continue;
        }
        let scoped =
            (parts.get(i + 1)).is_some_and(|p| p.as_os_str().to_string_lossy().starts_with('@'));
        let name: PathBuf = parts[i + 1..]
            .iter()
            .take(1 + usize::from(scoped))
            .collect();
        if parts.len() > i + 1 + name.components().count() {
            found.push((dir.join(&name), name));
        }
    }
    found
}
pub fn mix_deps(root: &Path, file: &Path) -> Vec<PathBuf> {
    file.ancestors()
        .take_while(|dir| dir.starts_with(root))
        .filter(|dir| dir.join("mix.exs").is_file())
        .map(|dir| dir.join("deps"))
        .filter(|dir| dir.is_dir())
        .collect()
}
#[rustfmt::skip]
const NODE_BUILTINS: &[&str] = &[
    "assert", "async_hooks", "buffer", "child_process", "cluster", "console", "constants", "crypto",
    "dgram", "diagnostics_channel", "dns", "domain", "events", "fs", "http", "http2", "https",
    "inspector", "module", "net", "os", "path", "perf_hooks", "process", "punycode", "querystring",
    "readline", "repl", "stream", "string_decoder", "sys", "timers", "tls", "trace_events", "tty",
    "url", "util", "v8", "vm", "wasi", "worker_threads", "zlib",
];
pub fn package_copy(
    root: &Path,
    roots: &[PathBuf],
    files: &[PathBuf],
    own: &[PathBuf],
    module: &[String],
) -> Option<PackageCopy> {
    let (name, name_parts) = match module {
        [scope, pkg, ..] if scope.starts_with('@') => (format!("{scope}/{pkg}"), 2),
        [pkg, ..] if !NODE_BUILTINS.contains(&pkg.as_str()) => (pkg.clone(), 1),
        _ => return Some(PackageCopy::default()),
    };
    let types = format!("@types/{}", name.trim_start_matches('@').replace('/', "__"));
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
        name_parts,
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
        let whole = copy
            .module(module)
            .is_some_and(|m| m.matched_parts == module.len());
        let exports = std::fs::read_to_string(level.join(&name).join("package.json"))
            .is_ok_and(|text| exports_map(&text));
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
    let typescript_reads = |f: &PathBuf| {
        f.extension()
            .is_some_and(|e| ["ts", "tsx", "mts", "cts"].iter().any(|t| e == *t))
    };
    let declarations = |d: &PathBuf| -> Vec<PathBuf> {
        files
            .iter()
            .filter(|f| typescript_reads(f) && in_copy(f, std::slice::from_ref(d)))
            .cloned()
            .collect()
    };
    if !copy.files.is_empty() && !copy.files.iter().any(typescript_reads) {
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
pub(super) fn exports_map(text: &str) -> bool {
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
#[derive(Debug, Default, PartialEq)]
pub struct PackageCopy {
    pub dirs: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
    pub name_parts: usize,
}
impl PackageCopy {
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
    pub fn module(&self, module: &[String]) -> Option<ModuleFiles<PathBuf>> {
        if self.files.is_empty() {
            return None;
        }
        let below: Vec<CopyFile> = self
            .files
            .iter()
            .map(|file| CopyFile {
                below_dir: self
                    .dirs
                    .iter()
                    .find_map(|d| file.strip_prefix(d).ok())
                    .unwrap_or(file),
                file,
            })
            .collect();
        let rest = module.get(self.name_parts..).unwrap_or_default();
        Some(match module_among(&below, rest, None) {
            Some(found) => ModuleFiles {
                matched_parts: self.name_parts + found.matched_parts,
                files: found.files.iter().map(|b| b.file.clone()).collect(),
            },
            None => ModuleFiles {
                matched_parts: self.name_parts,
                files: self.files.clone(),
            },
        })
    }
}
#[derive(Clone)]
struct CopyFile<'a> {
    below_dir: &'a Path,
    file: &'a PathBuf,
}
impl AsRef<Path> for CopyFile<'_> {
    fn as_ref(&self) -> &Path {
        self.below_dir
    }
}
pub fn in_copy(path: &Path, copy: &[PathBuf]) -> bool {
    copy.iter().any(|dir| {
        path.strip_prefix(dir)
            .is_ok_and(|rest| !rest.components().any(|c| c.as_os_str() == "node_modules"))
    })
}
pub fn external_files(kind: Kind, dirs: &[PathBuf]) -> Vec<PathBuf> {
    let go = kind == Kind::Go;
    let python = kind == Kind::Python;
    let spelled: Vec<PathBuf> = (dirs.iter())
        .filter(|d| !objc_frameworks(d))
        .cloned()
        .collect();
    let real = real_dirs(&spelled);
    let mut files = Vec::new();
    for dir in dirs {
        let copy_of_itself = dir.file_name().map(std::ffi::OsStr::to_owned);
        let others: Vec<PathBuf> = dirs.iter().filter(|o| *o != dir).cloned().collect();
        let frameworks = kind == Kind::C && objc_frameworks(dir);
        let sysroot = kind == Kind::Rust && dir.ends_with("lib/rustlib/src/rust/library");
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
                let untaken = sysroot && is_dir && {
                    let name = e.file_name().to_string_lossy();
                    name == "benches" || name.ends_with("tests")
                };
                let header = match e.depth() {
                    1 => e.file_name().to_string_lossy().ends_with(".framework"),
                    2 => e.file_name() == "Headers",
                    _ => true,
                };
                !unreachable
                    && !base_packages
                    && !nested
                    && !untaken
                    && (!frameworks || header)
                    && (e.depth() != 1 || Some(e.file_name()) != copy_of_itself.as_deref())
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
    if kind == Kind::C {
        let mut seen = std::collections::HashSet::new();
        files.retain(|f| seen.insert(f.clone()));
    }
    files
}
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
pub fn c_spelled(path: &Path, dirs: &[RealDir]) -> PathBuf {
    let Ok(real) = path.canonicalize() else {
        return path.to_path_buf();
    };
    dirs.iter()
        .find_map(|d| Some(d.spelled.join(real.strip_prefix(&d.links_followed).ok()?)))
        .unwrap_or_else(|| path.to_path_buf())
}
pub struct RealDir {
    pub spelled: PathBuf,
    pub links_followed: PathBuf,
}
pub fn real_dirs(dirs: &[PathBuf]) -> Vec<RealDir> {
    (dirs.iter())
        .filter_map(|d| {
            Some(RealDir {
                spelled: d.clone(),
                links_followed: d.canonicalize().ok()?,
            })
        })
        .collect()
}
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
const DEMOTED_DIRS: &[&str] = &[
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
];
const TEST_FILES: &[&str] = &[
    "test_*",
    "conftest.py",
    "*_test.*",
    "*_spec.*",
    "*.test.*",
    "*.spec.*",
];
const GENERATED_FILES: &[&str] = &[
    "*_pb2.py",
    "*_pb2_grpc.py",
    "*.pb.go",
    "*.gen.go",
    "*.generated.*",
];
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Declaration,
    Open,
    Code,
    Tests,
}
pub fn rank(path: &Path, here: Option<&Path>, declaration: bool) -> Rank {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let rows = [DEMOTED_DIRS, TEST_FILES, GENERATED_FILES];
    let demoted = rows
        .iter()
        .flat_map(|rows| rows.iter())
        .any(|row| match row.strip_suffix('/') {
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
    let tier = if demoted && here != Some(path) {
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
    Rank {
        tier,
        dirs_from_open_file: steps,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rank {
    pub tier: Tier,
    pub dirs_from_open_file: usize,
}
pub struct GoBuild {
    pub os: &'static str,
    pub arch: &'static str,
    pub cgo: bool,
    pub tags: Vec<String>,
}

impl GoBuild {
    pub fn host() -> Self {
        Self {
            os: match std::env::consts::OS {
                "macos" => "darwin",
                os => os,
            },
            arch: match std::env::consts::ARCH {
                "x86_64" => "amd64",
                "aarch64" => "arm64",
                "x86" => "386",
                arch => arch,
            },
            cgo: true,
            tags: Vec::new(),
        }
    }

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
pub fn go_built(path: &Path, text: &str, build: &GoBuild) -> GoBuilt {
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
        return GoBuilt::Excluded;
    }
    let literal = literal_lines(Kind::Go, text);
    let head = || {
        let lines = text.lines().take_while(|l| !l.starts_with("package "));
        lines
            .enumerate()
            .filter(|(i, _)| !literal[*i])
            .map(|(_, l)| l)
    };
    let Some(expr) = head().find_map(|l| l.strip_prefix("//go:build ")) else {
        return match head().any(|l| l.starts_with("// +build")) {
            true => GoBuilt::Unread,
            false => GoBuilt::Compiled,
        };
    };
    static TOKEN: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"&&|\|\||[!()]|[\w.]+").unwrap());
    let tokens: Vec<&str> = TOKEN.find_iter(expr).map(|m| m.as_str()).collect();
    type Tag<'a> = &'a dyn Fn(&str) -> bool;
    fn or_of_ands(t: &[&str], i: &mut usize, tag: Tag) -> Option<bool> {
        let mut value = and_of_unaries(t, i, tag)?;
        while t.get(*i) == Some(&"||") {
            *i += 1;
            value |= and_of_unaries(t, i, tag)?;
        }
        Some(value)
    }
    fn and_of_unaries(t: &[&str], i: &mut usize, tag: Tag) -> Option<bool> {
        let mut value = unary(t, i, tag)?;
        while t.get(*i) == Some(&"&&") {
            *i += 1;
            value &= unary(t, i, tag)?;
        }
        Some(value)
    }
    fn unary(t: &[&str], i: &mut usize, tag: Tag) -> Option<bool> {
        let token = *t.get(*i)?;
        *i += 1;
        match token {
            "!" => unary(t, i, tag).map(|v| !v),
            "(" => {
                let value = or_of_ands(t, i, tag);
                *i += 1;
                value
            }
            "&&" | "||" | ")" => None,
            name => Some(tag(name)),
        }
    }
    match or_of_ands(&tokens, &mut 0, &tag) {
        Some(true) => GoBuilt::Compiled,
        Some(false) => GoBuilt::Excluded,
        None => GoBuilt::Unread,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoBuilt {
    Compiled,
    Excluded,
    Unread,
}
