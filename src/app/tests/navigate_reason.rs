//! What `d` says about how it found the target, and the cut greps.

use super::*;

/// What `d` says about a lookup outside the project: the module an import or a path names,
/// and "by name" once the search is no longer inside it.
#[test]
fn a_module_lookup_says_which_module_or_that_it_went_by_name() {
    let (dir, mut a) = project_app(
        "outside",
        &[
            (
                "main.go",
                "package main\n\nimport (\n\t\"database/sql\"\n\t\"database/sql/pq\"\n\t\"errors\"\n\t\"github.com/foo/bar\"\n\t\"gopkg.in/yaml.v3\"\n\t\"example.com/kit\"\n)\n\nfunc main() {\n\t_ = yaml.Unmarshal(nil, nil)\n\tkit.Wire()\n\tbar.Baz()\n\tsql.Open(\"\", \"\")\n\t_ = errors.New(\"\")\n\tpq.Open(\"\")\n}\n",
            ),
            (
                "main.rs",
                "fn main() {\n    let s = String::new().into_owned();\n    path.join(\"y\");\n    std::fs::read_to_string(\"x\");\n}\n",
            ),
            (
                "m.py",
                "import os\nimport jsonx\n\nos.path.join('a', 'b')\njsonx.dumps_x(1)\n",
            ),
            // The project's own `dumps_x` is not the one `import jsonx` names.
            ("codec.py", "def dumps_x(o):\n    return o\n"),
            (
                "app.ts",
                "import { Hono } from 'hono';\n\nexport const app = new Hono();\n",
            ),
        ],
    );
    let root = external_root(
        "outside",
        &[
            (
                "gopkg.in/yaml.v3@v3.0.1/yaml.go",
                "package yaml\n\nfunc Unmarshal(in []byte, out any) error { return nil }\n",
            ),
            (
                "google.golang.org/protobuf@v1.34.0/proto/decode.go",
                "package proto\n\nfunc (o UnmarshalOptions) Unmarshal(b []byte, m any) error { return nil }\n",
            ),
            (
                "github.com/other/lib@v1.0.0/lib.go",
                "package lib\n\nfunc Baz() {}\n",
            ),
            // `kit` does not declare `Wire`: a package inside it is another package.
            (
                "example.com/kit@v1.0.0/kit.go",
                "package kit\n\nfunc Other() {}\n",
            ),
            (
                "example.com/kit@v1.0.0/inner/inner.go",
                "package inner\n\nfunc Wire() {}\n",
            ),
            (
                "github.com/else/thing@v1.0.0/thing.go",
                "package thing\n\nfunc Wire() {}\n",
            ),
            // A Go package is one directory: `database/sql` is not `database/sql/driver`,
            // and the standard library's `errors` is not a module's.
            (
                "src/database/sql/sql.go",
                "package sql\n\nfunc Open(driver, dsn string) {}\n",
            ),
            (
                "src/database/sql/driver/driver.go",
                "package driver\n\nfunc Open(name string) {}\n",
            ),
            (
                "src/errors/errors.go",
                "package errors\n\nfunc New(text string) error { return nil }\n",
            ),
            (
                "github.com/pkg/errors@v0.9.1/errors.go",
                "package errors\n\nfunc New(message string) error { return nil }\n",
            ),
            (
                "alloc/src/borrow.rs",
                "pub trait ToOwned {\n    fn into_owned(self) -> Self;\n}\n",
            ),
            (
                "std/src/path.rs",
                "impl Path {\n    pub fn join(&self, p: &str) {}\n}\n",
            ),
            ("std/src/fs.rs", "pub fn read_to_string(path: &str) {}\n"),
            ("os/__init__.py", ""),
            ("os/path.py", "def join(a, *p):\n    return a\n"),
            ("jsonx/a.py", "def dumps_x(o):\n    return o\n"),
            ("jsonx/b.py", "def dumps_x(o):\n    return o\n"),
            (
                "hono/dist/types/hono.d.ts",
                "export declare class Hono {\n}\n",
            ),
        ],
    );
    for kind in [Kind::Go, Kind::Rust, Kind::Python, Kind::TsJs] {
        use_roots(&mut a, kind, std::slice::from_ref(&root));
    }
    let at = |p: &str| format!("{}", root.join(p).display());
    for (file, code, want) in [
        // A Go package named without its module path's `.v3`.
        (
            "main.go",
            "yaml.Unmarshal",
            jump(
                "Unmarshal: via import gopkg.in/yaml.v3",
                &at("gopkg.in/yaml.v3@v3.0.1/yaml.go:3"),
            ),
        ),
        // `github.com/foo/bar` is not installed: `github.com/other/lib` only shares a prefix.
        (
            "main.go",
            "bar.Baz",
            jump(
                "Baz: by name, 1 match",
                &at("github.com/other/lib@v1.0.0/lib.go:3"),
            ),
        ),
        (
            "main.go",
            "sql.Open",
            jump(
                "Open: via import database/sql",
                &at("src/database/sql/sql.go:3"),
            ),
        ),
        (
            "main.go",
            "errors.New",
            jump("New: via import errors", &at("src/errors/errors.go:3")),
        ),
        // Nor is a package inside the imported one: a name it lacks is looked for everywhere.
        (
            "main.go",
            "kit.Wire",
            picker(
                "Wire: by name, 2 declarations",
                &[
                    ("Wire", "example.com/kit@v1.0.0/inner/inner.go:3"),
                    ("Wire", "github.com/else/thing@v1.0.0/thing.go:3"),
                ],
            ),
        ),
        // A package that is not installed: its parent directory is no proof.
        (
            "main.go",
            "pq.Open",
            picker(
                "Open: by name, 2 declarations",
                &[
                    ("Open", "src/database/sql/driver/driver.go:3"),
                    ("Open", "src/database/sql/sql.go:3"),
                ],
            ),
        ),
        // A Rust call on a value still looks outside the project.
        (
            "main.rs",
            ".into_owned",
            jump(
                "into_owned \u{2192} ToOwned::into_owned (by name, 1 match)",
                &at("alloc/src/borrow.rs:2"),
            ),
        ),
        // A value named like a module is found in that module, by name.
        (
            "main.rs",
            "path.join",
            jump(
                "join \u{2192} Path::join (by name, 1 match)",
                &at("std/src/path.rs:2"),
            ),
        ),
        (
            "main.rs",
            "std::fs::read_to_string",
            jump("read_to_string: via std::fs", &at("std/src/fs.rs:1")),
        ),
        (
            "m.py",
            "os.path.join",
            jump("join: via import os.path", &at("os/path.py:1")),
        ),
        // The name a TypeScript import takes is not a file of the package.
        (
            "app.ts",
            "new Hono",
            jump("Hono: via import hono", &at("hono/dist/types/hono.d.ts:1")),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    // Rows that share an import say so, and so does the title.
    d_on(&mut a, "m.py", "jsonx.dumps_x");
    let row = |place: &str| {
        (
            "dumps_x".to_string(),
            "via import jsonx".to_string(),
            place.to_string(),
        )
    };
    assert_eq!(
        shown(&mut a),
        Shown::Picker(
            "dumps_x: via import jsonx, 2 declarations".into(),
            vec![row("jsonx/a.py:1"), row("jsonx/b.py:1")],
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// `self.word` alone is the class's own member: a dependency's method of that name is not
/// a candidate.
#[test]
fn a_bare_self_stays_in_the_project() {
    let (dir, mut a) = project_app(
        "own",
        &[(
            "a.py",
            "class A:\n    def stop(self):\n        pass\n\n    def run(self):\n        self.stop()\n",
        )],
    );
    let root = external_root(
        "own",
        &[(
            "threading.py",
            "class Thread:\n    def stop(self):\n        pass\n",
        )],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
    d_on(&mut a, "a.py", "self.stop");
    assert_eq!(
        shown(&mut a),
        jump("stop \u{2192} A.stop (via self: A)", "a.py:2")
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// #104. A `self.word` whose class extends one outside the project gets the project's
/// declarations of the word and never the dependency's, although the dependency declares it.
#[test]
fn a_self_word_past_a_base_outside_the_project_stays_in_it() {
    let (dir, mut a) = project_app(
        "own-outside",
        &[(
            "a.py",
            "from threading import Thread\n\n\nclass A(Thread):\n    def run(self):\n        self.stop()\n",
        )],
    );
    let root = external_root(
        "own-outside",
        &[(
            "threading.py",
            "class Thread:\n    def stop(self):\n        pass\n",
        )],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&root));
    d_on(&mut a, "a.py", "self.stop");
    assert_eq!(shown(&mut a), jump("no definition for stop", "a.py:6"));
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// #100. A grep that stopped at the cap counts a lower bound also after a filter has made
/// the list short: the one top-level `Pick` left of a cut list is offered with a `+`, not
/// jumped to as the package's only one.
#[test]
fn a_count_behind_a_cut_grep_says_so() {
    let (dir, mut a) = project_app(
        "cut-count",
        &[(
            "main.go",
            "package main\n\nimport \"example.com/lib\"\n\nfunc main() {\n\tlib.Pick()\n}\n",
        )],
    );
    let methods = "func (o Row) Pick() {}\n".repeat(search::MAX_HITS);
    let root = external_root(
        "cut-count",
        &[(
            "example.com/lib@v1.0.0/lib.go",
            &format!("package lib\n\nfunc Pick() {{}}\n\n{methods}"),
        )],
    );
    use_roots(&mut a, Kind::Go, std::slice::from_ref(&root));
    d_on(&mut a, "main.go", "lib.Pick");
    assert_eq!(
        shown(&mut a),
        Shown::Picker(
            "Pick: via import example.com/lib, 1+ declarations".into(),
            vec![(
                "Pick".into(),
                "via import example.com/lib".into(),
                "example.com/lib@v1.0.0/lib.go:3".into()
            )],
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// A cut in a grep whose hits are all dropped (`x.String` looked for as `x`'s own `String`)
/// does not mark the members that are then listed: one match is a jump, as it was.
#[test]
fn a_cut_grep_that_is_dropped_marks_nothing() {
    let plain = "func String() {}\n".repeat(search::MAX_HITS);
    let (dir, mut a) = project_app(
        "cut-dropped",
        &[
            ("a/many.go", &format!("package a\n\n{plain}")),
            (
                "main.go",
                "package main\n\ntype T struct{}\n\nfunc (t T) String() {}\n\nfunc main() {\n\tx.String()\n}\n",
            ),
        ],
    );
    a.external
        .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "main.go", "x.String");
    assert_eq!(
        shown(&mut a),
        jump("String \u{2192} T.String (by name, 1 match)", "main.go:5")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #104. The field lines of a name are grepped apart from its methods: a file of object-literal
/// keys that fills the grep leaves the method a candidate, and the count says the field side
/// was cut, so a single candidate is offered rather than jumped to.
#[test]
fn a_field_grep_cut_at_the_cap_keeps_the_methods_and_says_so() {
    let keys = format!(
        "export const rows = {{\n{}}};\n",
        "  id: 1,\n".repeat(search::MAX_HITS)
    );
    let (dir, mut a) = project_app(
        "field-cap",
        &[
            ("a/rows.ts", keys.as_str()),
            (
                "z/model.ts",
                "export class Model {\n  id(): number {\n    return 1;\n  }\n}\n",
            ),
            (
                "main.ts",
                "export function f(u: any): number {\n  return u.id();\n}\n",
            ),
        ],
    );
    for kind in [Kind::Python, Kind::TsJs, Kind::Go] {
        a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
    }
    d_on(&mut a, "main.ts", "u.id");
    assert_eq!(
        shown(&mut a),
        picker(
            "id: by name, 1+ declarations",
            &[("Model.id", "z/model.ts:2")]
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// An editable install puts the project's own `src` on `sys.path`: a file the project walk
/// listed stays editable, while a gitignored `.venv` under the root does not.
#[test]
fn a_project_file_under_an_external_root_stays_editable() {
    let (dir, mut a) = project_app(
        "editable",
        &[
            (
                "src/app/repo.py",
                "class Repo:\n    def save(self):\n        pass\n",
            ),
            ("src/app/cli.py", "def main(r):\n    r.save()\n"),
            (".gitignore", ".venv\n"),
            (".venv/lib/site.py", "x = 1\n"),
        ],
    );
    use_roots(
        &mut a,
        Kind::Python,
        &[dir.join("src"), dir.join(".venv/lib")],
    );
    d_on(&mut a, "src/app/cli.py", "r.save");
    assert_eq!(
        shown(&mut a),
        jump(
            "save \u{2192} Repo.save (by name, 1 match)",
            "src/app/repo.py:2"
        )
    );
    assert_eq!(a.buf.readonly, None);
    a.jump_to(&dir.join(".venv/lib/site.py"), 1);
    assert_eq!(a.buf.readonly, Some("outside the project"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Edits that cannot be saved keep merl on the file; the status line must not claim `d` went.
#[test]
fn a_refused_jump_does_not_report_a_resolution() {
    let (dir, mut a) = project_app(
        "refused",
        &[
            ("a.py", "def helper():\n    pass\n"),
            ("b.py", "from a import helper\n\nhelper()\n"),
        ],
    );
    a.jump_to(&dir.join("b.py"), 3);
    (a.dirty, a.conflict) = (true, true);
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(at(&a), (dir.join("b.py"), 2));
    assert_eq!(a.message, "");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_jump_says_how_the_target_was_found() {
    let mut a = fixture_app("go");
    // A bare word declared at the top level: nothing to qualify it with.
    d_on(&mut a, "service.go", "\tHandleDelete");
    assert_eq!(a.message, "HandleDelete: by name, 1 match");
    assert!(a.line_str().starts_with("func HandleDelete("));
    let one = |reason| {
        vec![Candidate {
            hit: Hit {
                path: PathBuf::from("x"),
                line: 1,
                text: String::new(),
            },
            reason,
        }]
    };
    assert_eq!(
        resolution(
            "load",
            Some("json.load"),
            &one(Reason::Import("json".into())),
            None,
            false
        ),
        "load \u{2192} json.load (via import json)"
    );
    let mut two = one(Reason::ByName);
    two.extend(one(Reason::Path("std::fs".into())));
    assert_eq!(
        resolution("read", None, &two, None, false),
        "read: 2 declarations"
    );
    // The candidates stop at MAX_HITS: the count is a lower bound.
    let many: Vec<Candidate> = (0..search::MAX_HITS)
        .flat_map(|_| one(Reason::ByName))
        .collect();
    assert_eq!(
        resolution("save", None, &many, None, false),
        format!("save: by name, {}+ declarations", search::MAX_HITS)
    );
    // So is a list a search cut short, one candidate or more.
    assert_eq!(
        resolution("id", None, &one(Reason::ByName), None, true),
        "id: by name, 1+ declarations"
    );
    assert_eq!(
        resolution("read", None, &[], None, false),
        "no definition for read"
    );
    // A chain in front of the word that could not be followed says where it broke.
    assert_eq!(
        resolution(
            "remove",
            Some("UserService.remove"),
            &one(Reason::ByName),
            Some("users"),
            false
        ),
        "remove \u{2192} UserService.remove (by name, 1 match, chain broke at users)"
    );
    assert_eq!(
        resolution("read", None, &[], Some("repo"), false),
        "no definition for read (chain broke at repo)"
    );
}

#[test]
fn navigation_searches_the_open_file_as_it_is_on_screen() {
    let (dir, mut a) = project_app(
        "unsaved",
        &[(
            "lib.rs",
            "fn main() { target(); }\nfn target() {}\n// end\n",
        )],
    );
    a.jump_to(&dir.join("lib.rs"), 1);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Enter, KeyModifiers::NONE);
    press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
    // Undo from navigation: until autosave the buffer is a line shorter than the disk.
    press(&mut a, KeyCode::Char('z'), KeyModifiers::CONTROL);
    assert!(a.dirty);
    a.col = 12;
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(a.line_str(), "fn target() {}", "{}", a.message);
    std::fs::remove_dir_all(&dir).unwrap();
}
