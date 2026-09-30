//! Imports: the modules a name comes from and the files they reach.

use super::*;

#[test]
fn imports_bind_names_to_module_paths() {
    let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let py = "import json\nimport numpy as np\nimport os.path\nfrom collections import OrderedDict, deque as dq\nfrom . import local\nfrom ..models import Note\nfrom typing import (\n    Any,\n    Final,\n)\n";
    let got = imports(Kind::Python, py);
    assert_eq!(
        got,
        [
            ("json".into(), p(&["json"])),
            ("np".into(), p(&["numpy"])),
            ("os".into(), p(&["os"])),
            ("OrderedDict".into(), p(&["collections", "OrderedDict"])),
            ("dq".into(), p(&["collections", "deque"])),
            // A relative import is a project file, marked by its leading `.` part.
            ("local".into(), p(&[".", "local"])),
            // Each dot past the first is a directory up.
            ("Note".into(), p(&["..", "models", "Note"])),
            ("Any".into(), p(&["typing", "Any"])),
            ("Final".into(), p(&["typing", "Final"])),
        ]
    );
    // A comment inside the brackets is no name, and a `)` in it does not close them (#280).
    let py = "from .models import (\n    a,  # noqa: F401, (see #1)\n    b,\n)\nfrom x import c  # d, e\n";
    assert_eq!(
        imports(Kind::Python, py),
        [
            ("a".into(), p(&[".", "models", "a"])),
            ("b".into(), p(&[".", "models", "b"])),
            ("c".into(), p(&["x", "c"])),
        ]
    );
    // Nor after a plain `import` (#298).
    assert_eq!(
        imports(Kind::Python, "import a, b  # c, d\n"),
        [("a".into(), p(&["a"])), ("b".into(), p(&["b"]))]
    );
    let rs = "use std::fs;\nuse std::collections::{HashMap, hash_map::Entry};\nuse regex::Regex as Re;\nuse crate::buffer::Buffer;\npub(crate) use anyhow::{self, Context};\nuse std::{\n    io::Write,\n    path::Path,\n};\n";
    let got = imports(Kind::Rust, rs);
    assert_eq!(
        got,
        [
            ("fs".into(), p(&["std", "fs"])),
            ("HashMap".into(), p(&["std", "collections", "HashMap"])),
            (
                "Entry".into(),
                p(&["std", "collections", "hash_map", "Entry"])
            ),
            ("Re".into(), p(&["regex", "Regex"])),
            ("anyhow".into(), p(&["anyhow"])),
            ("Context".into(), p(&["anyhow", "Context"])),
            ("Write".into(), p(&["std", "io", "Write"])),
            ("Path".into(), p(&["std", "path", "Path"])),
        ]
    );
    let go = "package x\n\nimport \"strings\"\n\nimport (\n\t\"fmt\"\n\ttoml \"github.com/BurntSushi/toml\"\n\t\"github.com/go-chi/chi/v5\"\n\t\"gopkg.in/yaml.v3\"\n\t\"github.com/mattn/go-sqlite3\"\n\t\"github.com/nats-io/nats.go\"\n\t\"k8s.io/api/core/v1\"\n)\n";
    let got = imports(Kind::Go, go);
    assert_eq!(
        got,
        [
            ("strings".into(), p(&["strings"])),
            ("fmt".into(), p(&["fmt"])),
            ("toml".into(), p(&["github.com", "BurntSushi", "toml"])),
            ("chi".into(), p(&["github.com", "go-chi", "chi", "v5"])),
            ("v5".into(), p(&["github.com", "go-chi", "chi", "v5"])),
            // The package name without the decorations its module path carries.
            ("yaml".into(), p(&["gopkg.in", "yaml.v3"])),
            ("sqlite3".into(), p(&["github.com", "mattn", "go-sqlite3"])),
            ("nats".into(), p(&["github.com", "nats-io", "nats.go"])),
            ("core".into(), p(&["k8s.io", "api", "core", "v1"])),
            ("v1".into(), p(&["k8s.io", "api", "core", "v1"])),
        ]
    );
    let ts = "import fs from 'node:fs';\nimport { join, resolve as res } from \"path\";\nimport * as React from 'react';\nimport type { Foo } from '@scope/pkg/sub';\nimport local from './local';\nconst chalk = require('chalk');\nconst { a, b } = await import('lib');\nimport cp = require('child_process');\nconst utils = require('../lib/utils');\nimport def, { type Bar, baz as qux } from './mixed';\n";
    let got = imports(Kind::TsJs, ts);
    assert_eq!(
        got,
        [
            // The last part is what the import takes: the default export, a name, or the
            // whole module.
            // `node:` stays: the module is Node's own, whatever is installed under its name.
            ("fs".into(), p(&["node:fs", "default"])),
            ("join".into(), p(&["path", "join"])),
            ("res".into(), p(&["path", "resolve"])),
            ("React".into(), p(&["react", "*"])),
            ("Foo".into(), p(&["@scope", "pkg", "sub", "Foo"])),
            ("local".into(), p(&[".", "local", "default"])),
            ("chalk".into(), p(&["chalk", "*"])),
            ("a".into(), p(&["lib", "a"])),
            ("b".into(), p(&["lib", "b"])),
            ("cp".into(), p(&["child_process", "*"])),
            ("utils".into(), p(&["..", "lib", "utils", "*"])),
            ("def".into(), p(&[".", "mixed", "default"])),
            ("Bar".into(), p(&[".", "mixed", "Bar"])),
            ("qux".into(), p(&[".", "mixed", "baz"])),
        ]
    );
}

#[test]
fn module_files_are_the_project_files_an_import_names() {
    let (dir, files) = scratch(
        "modules",
        &[
            // Python: a src layout, a package, a module inside a package.
            ("src/app/__init__.py", ""),
            ("src/app/repos.py", ""),
            ("src/app/json.py", ""),
            ("src/app/store/__init__.py", ""),
            ("src/app/store/backends/memory.py", ""),
            // TypeScript: `paths` and `baseUrl` in the config a nested one extends.
            (
                "tsconfig.base.json",
                "{\n  // shared\n  \"compilerOptions\": {\n    \"baseUrl\": \"web\",\n    \"paths\": {\"@/*\": [\"src/*\"], \"@lib\": [\"lib/index.ts\"]},\n  },\n}\n",
            ),
            (
                "web/tsconfig.json",
                "{ \"$schema\": \"https://json.schemastore.org/tsconfig\", \"extends\": \"../tsconfig.base\" }\n",
            ),
            ("web/src/page.ts", ""),
            ("web/src/x.ts", ""),
            ("web/src/x.js", ""),
            ("web/src/types.d.ts", ""),
            ("web/src/ui/index.tsx", ""),
            ("web/lib/index.ts", ""),
            // Go: a module nested in another.
            ("go.mod", "module example.com/app\n\ngo 1.22\n"),
            ("internal/repo/repo.go", ""),
            ("internal/repo/repo_test.go", ""),
            ("internal/repo/sub/sub.go", ""),
            (
                "tools/go.mod",
                "module example.com/app/tools // generators\n",
            ),
            ("tools/gen/gen.go", ""),
        ],
    );
    let found = |kind, here: &str, module: &[&str]| -> Vec<String> {
        let module: Vec<String> = module.iter().map(|s| s.to_string()).collect();
        module_files(kind, &dir, &files, Path::new(here), &module)
            .iter()
            .map(|f| f.display().to_string())
            .collect()
    };
    let (py, ts, go) = (Kind::Python, Kind::TsJs, Kind::Go);
    assert_eq!(
        found(py, "main.py", &["app", "repos"]),
        ["src/app/repos.py"]
    );
    assert_eq!(
        found(py, "main.py", &["app", "store"]),
        ["src/app/store/__init__.py"]
    );
    // A module inside the `app` package is `app.json`, not the top-level `json`.
    assert!(found(py, "main.py", &["json"]).is_empty());
    let memory = "src/app/store/backends/memory.py";
    assert_eq!(found(py, memory, &[".."]), ["src/app/store/__init__.py"]);
    assert_eq!(found(py, memory, &["...", "repos"]), ["src/app/repos.py"]);
    assert!(found(py, "main.py", &["..", "repos"]).is_empty());
    let page = "web/src/page.ts";
    // The TypeScript file before the JavaScript one, also behind a `.js` specifier.
    assert_eq!(found(ts, page, &[".", "x"]), ["web/src/x.ts"]);
    assert_eq!(found(ts, page, &[".", "x.js"]), ["web/src/x.ts"]);
    assert_eq!(found(ts, page, &[".", "types"]), ["web/src/types.d.ts"]);
    assert_eq!(found(ts, page, &[".", "ui"]), ["web/src/ui/index.tsx"]);
    assert_eq!(found(ts, page, &["@", "ui"]), ["web/src/ui/index.tsx"]);
    assert_eq!(found(ts, page, &["@lib"]), ["web/lib/index.ts"]);
    // A name no alias matches, under `baseUrl`.
    assert_eq!(found(ts, page, &["src", "x"]), ["web/src/x.ts"]);
    assert!(found(ts, page, &["react"]).is_empty());
    assert!(found(ts, "page.ts", &["..", "x"]).is_empty());
    assert_eq!(
        found(go, "main.go", &["example.com", "app", "internal", "repo"]),
        ["internal/repo/repo.go"]
    );
    assert_eq!(
        found(go, "main.go", &["example.com", "app", "tools", "gen"]),
        ["tools/gen/gen.go"]
    );
    assert!(found(go, "main.go", &["example.com", "application"]).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn in_module_follows_the_parts_through_versions_and_escapes() {
    let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let m = |path: &str, parts: &[&str]| in_module(Path::new(path), &p(parts));
    assert!(m("/lib/python3.13/json/__init__.py", &["json"]));
    assert!(!m("/lib/python3.13/jsonschema/x.py", &["json"]));
    assert!(m("/rust/library/std/src/fs.rs", &["std", "fs"]));
    assert!(m(
        "/registry/src/idx/grep-regex-0.1.13/src/lib.rs",
        &["grep_regex"]
    ));
    assert!(!m(
        "/registry/src/idx/grep-searcher-0.1.13/src/lib.rs",
        &["grep_regex"]
    ));
    assert!(m(
        "/mod/github.com/!burnt!sushi/toml@v1.4.0/decode.go",
        &["github.com", "BurntSushi", "toml"]
    ));
    assert!(m("/go/src/strings/builder.go", &["strings"]));
    assert!(m("/node_modules/@types/node/fs.d.ts", &["fs"]));
    assert!(m(
        "/node_modules/@scope/pkg/sub/index.d.ts",
        &["@scope", "pkg", "sub"]
    ));
    assert!(m(
        "/node_modules/@types/scope__pkg/sub.d.ts",
        &["@scope", "pkg", "sub"]
    ));
    // The types of a scoped package are not an unscoped namesake's, nor another scope's, and
    // only `@types` spells a scope so.
    assert!(!m(
        "/node_modules/@types/babel__traverse/index.d.ts",
        &["traverse"]
    ));
    assert!(!m(
        "/node_modules/@types/other__pkg/index.d.ts",
        &["@scope", "pkg"]
    ));
    assert!(!m(
        "/node_modules/@types/scope__other/index.d.ts",
        &["@scope", "pkg"]
    ));
    assert!(!m(
        "/node_modules/scope__pkg/index.d.ts",
        &["@scope", "pkg"]
    ));
}

#[test]
fn typescript_spellings_are_read_in_typescript_only() {
    let lines = vec![
        "    repo".to_owned(),
        "        .find(1)?.name!.x".to_owned(),
    ];
    // A line led by a dot, `?.`, `!.` and a `#` are what they are in every other language: a
    // Python chain in brackets, Rust's `?`, a C `#define`.
    for kind in [Kind::Python, Kind::Go, Kind::Rust, Kind::C] {
        assert_eq!(unbroken(kind, &lines, 1, 9), None);
        assert_eq!(plain_access(kind, &lines[1], 24), (lines[1].clone(), 24));
        let word = definition_word(Some(kind), "#define LIMIT", 1);
        assert_eq!(word, Some((1..7, "define")));
    }
    assert_eq!(
        unbroken(Kind::TsJs, &lines, 1, 9),
        Some(("    repo.find(1)?.name!.x".to_owned(), 9))
    );
    assert_eq!(
        plain_access(Kind::TsJs, "    repo.find(1)?.name!.x", 24),
        ("    repo.find(1).name.x".to_owned(), 22)
    );
    // The forms a destructuring is written in, and what is none.
    let field = |from: &[&str], name: &str| {
        Some(Value::Field(
            from.iter().map(|s| s.to_string()).collect(),
            name.into(),
        ))
    };
    for (t, want) in [
        ("let { repo } = this", field(&["this"], "repo")),
        (
            "export const { a, repo } = deps.inner;",
            field(&["deps", "inner"], "repo"),
        ),
        (
            "var { users: repo } = this.#uow",
            field(&["this", "#uow"], "users"),
        ),
        ("const { repo = spare } = this;", None),
        ("const { ...repo } = this;", None),
        ("const { inner: { repo } } = this;", None),
        ("const { repo } = make();", None),
    ] {
        assert_eq!(ts_destructured(t, "repo"), want, "{t}");
    }
    // A barrel's list wrapped by prettier, and `export type`.
    let list =
        "export type {\n  Other,\n  Notifier,\n} from \"./b\";\nexport * as ns from \"./c\";\n";
    assert_eq!(
        reexports(list, "Notifier"),
        [vec![".".to_owned(), "b".to_owned()]]
    );
    assert_eq!(reexports(list, "ns"), Vec::<Vec<String>>::new());
    let alias =
        "export {\n  Hatch,\n  type Trunk as TrunkBase,\n};\nexport { A as B } from \"./x\";\n";
    assert_eq!(exported_as(alias, "TrunkBase"), Some("Trunk".to_owned()));
    assert_eq!(exported_as(alias, "B"), None);
    // A header with brackets and a `{}` default in it ends at the `{` of its body.
    let header = [
        "class Vault<S = {}>",
        "  extends mixin(Crate, { sealed: true })",
        "  implements Sealable",
        "{",
    ];
    assert_eq!(
        ts_header(&header, 0),
        (
            "class Vault extends mixin(Crate, { sealed: true }) implements Sealable {".to_owned(),
            3
        )
    );
}

#[test]
fn node_modules_are_those_from_the_file_up_to_the_root() {
    let dir = std::env::temp_dir().join(format!("merl-nm-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let root = dir.join("project");
    for d in [
        "node_modules",
        "project/node_modules",
        "project/api/node_modules",
        "project/web/src",
    ] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    assert_eq!(
        node_modules(&root, &root.join("api/src")),
        [root.join("api/node_modules"), root.join("node_modules")]
    );
    assert_eq!(
        node_modules(&root, &root.join("web/src")),
        [root.join("node_modules")]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn a_package_is_the_copy_in_the_nearest_node_modules_that_has_it() {
    let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let dir = std::env::temp_dir().join(format!("merl-copy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for f in [
        "real/node_modules/lib/index.d.ts",
        "real/node_modules/lib/extra.d.ts",
        "real/node_modules/typed/index.js",
        "real/node_modules/@scope/pkg/index.d.ts",
        "real/node_modules/.pnpm/pinned@1.0.0/node_modules/pinned/index.d.ts",
        "real/node_modules/.pnpm/pinned@2.0.0/node_modules/pinned/index.d.ts",
        "real/api/node_modules/lib/index.d.ts",
        "real/api/node_modules/@types/typed/index.d.ts",
        "real/api/node_modules/@types/scope__pkg/index.d.ts",
        "real/packages/shared/index.ts",
    ] {
        std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
        std::fs::write(dir.join(f), "").unwrap();
    }
    std::fs::create_dir_all(dir.join("real/node_modules/@app")).unwrap();
    let link = |to: &str, at: &str| std::os::unix::fs::symlink(to, dir.join(at)).unwrap();
    link(
        ".pnpm/pinned@2.0.0/node_modules/pinned",
        "real/node_modules/pinned",
    );
    link("../../packages/shared", "real/node_modules/@app/shared");
    // The project and its roots are spelled through a link, as a temporary directory is on
    // macOS, and so is the copy; the directory above is called as a path in a package is.
    std::fs::create_dir_all(dir.join("extra")).unwrap();
    link("../real", "extra/link");
    let root = dir.join("extra/link");
    let (api, top) = (root.join("api/node_modules"), root.join("node_modules"));
    let roots = [api.clone(), top.clone()];
    let files = external_files(Kind::TsJs, &roots);
    let own = [PathBuf::from("packages/shared/index.ts")];
    let copy = |module: &[&str]| {
        package_copy(&root, &roots, &files, &own, &p(module)).map(|c| {
            let mut files = c.files;
            files.sort();
            files
        })
    };
    // The nearest copy, unless only a farther one has the whole path, which Node goes on to.
    assert_eq!(
        copy(&["lib", "sub"]),
        Some(vec![api.join("lib/index.d.ts")])
    );
    assert_eq!(
        copy(&["lib", "extra"]),
        Some(vec![top.join("lib/extra.d.ts"), top.join("lib/index.d.ts")])
    );
    assert_eq!(
        copy(&["typed"]),
        Some(vec![api.join("@types/typed/index.d.ts")])
    );
    assert_eq!(
        copy(&["@scope", "pkg"]),
        Some(vec![api.join("@types/scope__pkg/index.d.ts")])
    );
    assert_eq!(
        copy(&["pinned"]),
        Some(vec![
            top.join(".pnpm/pinned@2.0.0/node_modules/pinned/index.d.ts")
        ])
    );
    assert_eq!(copy(&["fs"]), Some(vec![]));
    // A workspace package linked in is the project's own.
    assert_eq!(copy(&["@app", "shared"]), None);
    // A file of the copy, and one of a package it depends on.
    let copy = [top.join("lib")];
    assert!(in_copy(&top.join("lib/dist/index.d.ts"), &copy));
    assert!(!in_copy(
        &top.join("lib/node_modules/dep/index.d.ts"),
        &copy
    ));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_package_is_missing_when_nothing_installs_or_declares_it() {
    let dir = std::env::temp_dir().join(format!("merl-missing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let root = dir.join("project");
    let files: Vec<PathBuf> = [
        "tsconfig.json",
        "packages/labels/package.json",
        "web/src/app.ts",
        "web/node_modules/installed/index.d.ts",
        "node_modules/@types/typed/index.d.ts",
        "node_modules/@scope/pkg/index.d.ts",
        "types/ambient.d.ts",
        "js/jsconfig.json",
        "js/src/components/Button.js",
        "base/tsconfig.json",
        "base/src/components/Button.ts",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    for f in &files {
        std::fs::create_dir_all(root.join(f).parent().unwrap()).unwrap();
        std::fs::write(root.join(f), "").unwrap();
    }
    std::fs::write(
        root.join("tsconfig.json"),
        r#"{ "compilerOptions": { "paths": { "@server/*": ["./server/*"] } } }"#,
    )
    .unwrap();
    std::fs::write(
        root.join("packages/labels/package.json"),
        r#"{ "name": "@post/labels", "version": "1.0.0" }"#,
    )
    .unwrap();
    std::fs::write(
        root.join("types/ambient.d.ts"),
        "declare module 'untyped-lib' {}\ndeclare module \"*.svg\";\n",
    )
    .unwrap();
    std::fs::write(
        root.join("js/jsconfig.json"),
        r#"{ "compilerOptions": { "paths": { "@components/*": ["src/components/*"] } } }"#,
    )
    .unwrap();
    std::fs::write(
        root.join("base/tsconfig.json"),
        r#"{ "compilerOptions": { "baseUrl": "src" } }"#,
    )
    .unwrap();
    // Above the project, as Node looks there too.
    std::fs::create_dir_all(dir.join("node_modules/hoisted")).unwrap();
    let missing_from = |dir: &str, spec: &str| {
        let module: Vec<String> = spec.split('/').map(str::to_owned).collect();
        package_missing(&root, &files, Path::new(dir), &module)
    };
    let missing = |spec: &str| missing_from("web/src", spec);
    // What the project supplies: a `declare module`, an alias of `jsconfig.json`, `baseUrl`.
    assert!(!missing("untyped-lib"));
    assert!(!missing("icons/logo.svg"));
    assert!(missing("untyped-lib/sub"));
    assert!(!missing_from("js/src", "@components/Button"));
    assert!(missing_from("js/src", "@widgets/Button"));
    assert!(!missing_from("base/src/pages", "components/Button"));
    assert!(missing_from("base/src/pages", "widgets/Button"));
    assert!(missing("mobx-react"));
    assert!(missing("@other/pkg"));
    assert!(missing("es-toolkit/compat"));
    for here in [
        ".",
        "..",
        "@",
        "~",
        "#lib",
        "node:fs",
        "virtual:pwa",
        "fs",
        "@server/models",
        "@post/labels",
        "installed",
        "typed",
        "@scope/pkg",
        "hoisted",
    ] {
        assert!(!missing(&format!("{here}/x")), "{here}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_ts_import_line_is_the_line_of_the_name() {
    let text = "import a from \"a\";\n\nimport {\n  observable,\n  obs as seen,\n} from \"mobx\";\nconst req = require(\"x\");\n";
    assert_eq!(ts_import_line(text, "a"), Some(1));
    assert_eq!(ts_import_line(text, "seen"), Some(5));
    assert_eq!(ts_import_line(text, "observable"), Some(4));
    // `obs` is taken under another name, and `observable` only starts with it.
    assert_eq!(ts_import_line(text, "obs"), None);
    assert_eq!(ts_import_line(text, "req"), Some(7));
    assert_eq!(ts_import_line(text, "nothing"), None);
}

#[test]
fn a_require_binds_only_the_module_itself_or_a_name_of_it() {
    let text = "const { a, b: c } = require(\"./m\");\nconst d = require(\"debug\")(\"app\");\nconst e = require(\"./m\").e;\nconst f = require(\"./m\").create();\nconst g = require(\"./g\"),\n  h = require(\"./h\"),\n  LIMIT = 10;\nh = require(\"./i\");\n";
    let got = imports(Kind::TsJs, text);
    let path = |p: &[&str]| p.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(
        got,
        vec![
            ("a".to_owned(), path(&[".", "m", "a"])),
            ("c".to_owned(), path(&[".", "m", "b"])),
            ("e".to_owned(), path(&[".", "m", "e"])),
            ("g".to_owned(), path(&[".", "g", "*"])),
            ("h".to_owned(), path(&[".", "h", "*"])),
        ]
    );
    assert_eq!(ts_import_line(text, "h"), Some(6));
}

#[test]
fn module_exports_is_one_assignment_or_nothing_known() {
    assert_eq!(
        module_exports("class S {}\n\nmodule.exports = S;\n"),
        Some(Some((3, Some("S".to_owned()))))
    );
    assert_eq!(
        module_exports("module.exports = {\n  a,\n};\n"),
        Some(Some((1, None)))
    );
    assert_eq!(module_exports("exports.a = a;\n"), Some(None));
    assert_eq!(
        module_exports("module.exports = a;\nmodule.exports = b;\n"),
        Some(None)
    );
    assert_eq!(module_exports("export default a;\n"), None);
}

#[test]
fn external_files_ignore_no_gitignore_and_keep_the_kind() {
    let dir = std::env::temp_dir().join(format!("merl-ext-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("pkg")).unwrap();
    std::fs::write(dir.join(".gitignore"), "pkg\n").unwrap();
    std::fs::write(dir.join("pkg/mod.py"), "").unwrap();
    std::fs::write(dir.join("pkg/mod.pyi"), "").unwrap();
    std::fs::write(dir.join("pkg/mod.so"), "").unwrap();
    let mut files = external_files(Kind::Python, std::slice::from_ref(&dir));
    files.sort();
    assert_eq!(files, [dir.join("pkg/mod.py"), dir.join("pkg/mod.pyi")]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn external_go_files_are_what_an_import_reaches() {
    let dir = std::env::temp_dir().join(format!("merl-ext-go-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for f in [
        "strings/strings.go",
        "strings/strings_test.go",
        "go/parser/testdata/x.go",
        "cmd/go.mod",
        "cmd/compile/main.go",
    ] {
        std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
        std::fs::write(dir.join(f), "").unwrap();
    }
    assert_eq!(
        external_files(Kind::Go, std::slice::from_ref(&dir)),
        [dir.join("strings/strings.go")]
    );
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(declaration_file(Path::new(
        "node_modules/@types/node/fs.d.ts"
    )));
    assert!(declaration_file(Path::new("x/index.d.mts")));
    assert!(!declaration_file(Path::new("x/index.mjs")));
    assert!(!declaration_file(Path::new("x/index.ts")));
}

/// #227. The file of the crate module a top-level `use crate::…` or `use super::…` takes a name
/// from, at the paths a module's file sits at by default.
#[test]
fn rust_use_files_follow_the_crate_and_super_paths() {
    let files: Vec<PathBuf> = [
        "Cargo.toml",
        "src/lib.rs",
        "src/store.rs",
        "src/net/mod.rs",
        "src/net/store.rs",
        "src/net/cache/mod.rs",
        "src/bin/tool.rs",
        "loose/src/store.rs",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    let find = |here: &str, text: &str| -> Vec<String> {
        rust_use_files(&files, Path::new(here), text, "Cache")
            .iter()
            .map(|p| p.display().to_string())
            .collect()
    };
    let cases: [(&str, &str, &[&str]); 13] = [
        ("src/lib.rs", "use crate::store::Cache;", &["src/store.rs"]),
        (
            "src/net/client.rs",
            "use crate::net::cache::Cache;",
            &["src/net/cache/mod.rs"],
        ),
        (
            "src/net/client.rs",
            "use super::store::Cache;",
            &["src/net/store.rs"],
        ),
        (
            "src/net/client.rs",
            "use super::super::store::Cache;",
            &["src/store.rs"],
        ),
        // `mod.rs` is its directory's module: its `super` is the one above.
        (
            "src/net/mod.rs",
            "use super::store::Cache;",
            &["src/store.rs"],
        ),
        (
            "src/net/mod.rs",
            "pub use crate::{store::{self, Cache}};",
            &["src/store.rs"],
        ),
        ("src/store.rs", "use crate::Cache;", &["src/lib.rs"]),
        // Above the crate root, in a binary's own crate, outside a `Cargo.toml`'s `src/`.
        ("src/lib.rs", "use super::store::Cache;", &[]),
        ("src/bin/tool.rs", "use crate::store::Cache;", &[]),
        ("loose/src/lib.rs", "use crate::store::Cache;", &[]),
        // Bound twice, under another name, or inside an inline `mod`, whose `super` is not the
        // file's: `super::super` in `client.rs`'s tests is `net`, not the crate root.
        (
            "src/lib.rs",
            "use crate::store::Cache;\nfn f() {\n    use crate::net::store::Cache;\n}",
            &[],
        ),
        ("src/lib.rs", "use crate::store::Cache as Stored;", &[]),
        (
            "src/net/client.rs",
            "mod tests {\n    use super::super::store::Cache;\n}",
            &[],
        ),
    ];
    for (here, text, want) in cases {
        assert_eq!(find(here, text), want, "{here}: {text}");
    }
}

/// #333. A word in the module path of a Python import line is the module up to that word; an
/// imported name, an alias and any other line are not.
#[test]
fn a_word_in_a_python_import_path_is_its_module() {
    let module = |line: &str, word: &str| {
        let at = line.find(word).unwrap();
        python_import_module(line, at)
    };
    let parts = |p: &[&str]| Some(p.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let line = "from app.repos import UserRepo";
    assert_eq!(module(line, "app"), parts(&["app"]));
    assert_eq!(module(line, "repos"), parts(&["app", "repos"]));
    assert_eq!(module(line, "UserRepo"), None);
    let line = "from ..x.y import (";
    assert_eq!(module(line, "x"), parts(&["..", "x"]));
    let line = "import a.b as c, d.e  # f";
    assert_eq!(module(line, "b"), parts(&["a", "b"]));
    assert_eq!(module(line, "c"), None);
    assert_eq!(module(line, "e"), parts(&["d", "e"]));
    assert_eq!(module(line, "f"), None);
    assert_eq!(module("x = json.dumps", "json"), None);
}

#[test]
fn java_and_kotlin_imports_bind_their_names_to_paths() {
    let text = "package app.c;\n\nimport app.a.User;\nimport java.util.*;\nimport static a.b.Queries.isNull;\nimport static a.b.Queries.*;\nimport app.model.Topic as Model\nimport app.ui.fn // a function\n";
    let path = |p: &str| p.split('.').map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(
        jvm_imports(text),
        [
            ("User".to_owned(), path("app.a.User")),
            ("*".to_owned(), path("java.util.*")),
            ("isNull".to_owned(), path("a.b.Queries.isNull")),
            ("*".to_owned(), path("a.b.Queries.*")),
            ("Model".to_owned(), path("app.model.Topic")),
            ("fn".to_owned(), path("app.ui.fn")),
        ]
    );
    assert_eq!(jvm_package(text).as_deref(), Some("app.c"));
    assert_eq!(jvm_package("import a.B\n"), None);
}
