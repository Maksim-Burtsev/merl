use std::path::{Path, PathBuf};

use regex::Regex;

use super::*;

const PY: &str = "class Invoice:\n    def total(self):\n        return 0\n\n\ndef parse(t):\n    return Invoice()\n\n\nDEFAULT_LIMIT = 10\ntotal_foobar = 1\nprint(total_foobar, DEFAULT_LIMIT)\nNAME_RE: Final[re.Pattern[str]] = re.compile(r\"x\")\n";
const RS: &str = "pub struct Order<T> {\n    items: Vec<T>,\n}\n\nimpl<T> Order<T> {\n    pub fn sum(&self) -> u32 { 0 }\n}\n\npub(crate) const MAX_ORDERS: usize = 10;\n\npub async fn parse_order(s: &str) -> Order<u8> {\n    let mut order = Order { items: vec![] };\n    order.items.push(1);\n    order\n}\n\nmacro_rules! order {\n    () => {};\n}\n\nmod orders;\n";
const TS: &str = "export interface Order {\n  id: Id;\n}\n\nexport type Id = string;\n\nconst enum Status {\n  Open,\n}\n\nexport default class OrderService {\n  private cache = new Map();\n\n  async load(id: Id): Promise<Order> {\n    render(o);\n    return parse(id);\n  }\n\n  sum = (o: Order) => 0;\n}\n\nexport const parseOrder = (s: string): Order => JSON.parse(s);\n\nexport function render(o: Order) {\n  const n = 1;\n}\n\nfunction* ids() {}\n";
const JS: &str = "const helpers = {\n  parse(s) {\n    return s;\n  },\n  format: function (o) {\n    return o;\n  },\n};\nmodule.exports = helpers;\n";
const GO: &str = "package main\n\ntype Invoice struct{}\n\nfunc (i Invoice) Total() int { return 0 }\n\nfunc Parse(s string) Invoice { return Invoice{} }\n\nconst Limit = 10\n\nfunc main() {\n\tinv := Parse(\"x\")\n}\n";

/// A throwaway project on disk; grep needs real files.
fn scratch(tag: &str, files: &[(&str, &str)]) -> (PathBuf, Vec<PathBuf>) {
    let dir = std::env::temp_dir().join(format!("merl-grep-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, text) in files {
        std::fs::create_dir_all(dir.join(name).parent().unwrap()).unwrap();
        std::fs::write(dir.join(name), text).unwrap();
    }
    (dir, files.iter().map(|(n, _)| PathBuf::from(n)).collect())
}

fn project(tag: &str) -> (PathBuf, Vec<PathBuf>) {
    scratch(
        tag,
        &[
            ("a.py", PY),
            ("b.go", GO),
            ("c.rs", RS),
            ("d.ts", TS),
            ("e.js", JS),
        ],
    )
}

fn grep(dir: &Path, files: &[PathBuf], pat: &str, word: bool, smart: bool) -> Vec<Hit> {
    grep_project(dir, files, pat, word, smart, None, None).unwrap()
}

fn lines(hits: &[Hit]) -> Vec<(String, usize)> {
    hits.iter()
        .map(|h| (h.path.display().to_string(), h.line))
        .collect()
}

/// The lines `d`'s patterns match for `word` in `files`.
fn defs(dir: &Path, files: &[PathBuf], kind: Kind, word: &str) -> Vec<usize> {
    let pat = def_patterns(kind, word).join("|");
    grep(dir, files, &pat, false, false)
        .iter()
        .map(|h| h.line)
        .collect()
}

#[test]
fn python_def_patterns_find_declarations_only() {
    let (dir, files) = project("py");
    let py = files[..1].to_vec();
    // The `def` line, not the `total_foobar` assignment.
    assert_eq!(defs(&dir, &py, Kind::Python, "total"), [2]);
    assert_eq!(defs(&dir, &py, Kind::Python, "parse"), [6]);
    // `^W\s*(:[^=]*)?=` catches module constants, annotated or not.
    assert_eq!(defs(&dir, &py, Kind::Python, "DEFAULT_LIMIT"), [10]);
    assert_eq!(defs(&dir, &py, Kind::Python, "NAME_RE"), [13]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The lines `d`'s member patterns match for `word` in `files`: what `x.word` reaches.
fn members(dir: &Path, files: &[PathBuf], kind: Kind, word: &str) -> Vec<usize> {
    let pat = member_patterns(kind, word).unwrap().join("|");
    grep(dir, files, &pat, false, false)
        .iter()
        .map(|h| h.line)
        .collect()
}

const PY_MEMBERS: &str = "import asyncio\n\n\nclass UserRepository:\n    async def delete_user(self, user_id: int) -> None:\n        pass\n\n    def find_user(self, user_id):\n        return user_id\n\n\ndef find_user(user_id):\n    return user_id\n\n\nasync def main():\n    pass\n\n\ndelete_user = None\n";

#[test]
fn python_members_are_indented_defs_sync_or_async() {
    let (dir, files) = scratch("py-members", &[("repos.py", PY_MEMBERS)]);
    // `async def` is a declaration for a bare word too, at the top level or in a class.
    assert_eq!(defs(&dir, &files, Kind::Python, "delete_user"), [5, 20]);
    assert_eq!(defs(&dir, &files, Kind::Python, "main"), [16]);
    // `x.delete_user` reaches the method, not the module-level name of the same spelling.
    assert_eq!(members(&dir, &files, Kind::Python, "delete_user"), [5]);
    // `x.find_user` reaches the method; the module-level function takes an import.
    assert_eq!(members(&dir, &files, Kind::Python, "find_user"), [8]);
    assert_eq!(defs(&dir, &files, Kind::Python, "find_user"), [8, 12]);
    std::fs::remove_dir_all(&dir).unwrap();
}

const TS_MEMBERS: &str = "export interface Repo {\n  deleteUser(id: string): Promise<void>;\n  findUser?<T>(id: string): T | undefined\n  onChange: (id: string) => void;\n  name: string;\n}\n\nexport abstract class Base {\n  abstract deleteUser(id: string): Promise<void>;\n  get size(): number;\n}\n\nconst deleteUser = (id: string) => id;\nfindUser(id);\nrun(x).then((y): void => y);\nconst n = cond ? findUser(a) : b;\nexport const helpers = {\n  deleteUser(id) {\n    return id;\n  },\n};\nappend(target, visitor ? visitNode(s) : s);\nlog(\"(while reading XRef): \" + e);\ndeclare class Emitter {\n  on(event: string, cb: (x: T) => void): this;\n  append(...items: string[]): void;\n}\nclass Session {\n  close(): void {}\n}\nnoop(() => {})\nclass Svc {\n  constructor(\n    @Inject(W) private worker: Worker,\n  ) {}\n}\nclass One {\n  constructor(private readonly inline: Dep, public other: Dep) {}\n}\ndeclare class Wide {\n  pong<T extends Record<string, number>>(x: T): T;\n}\n";

#[test]
fn ts_members_include_signatures_without_a_body() {
    let (dir, files) = scratch("ts-members", &[("repo.ts", TS_MEMBERS)]);
    let m = |w| members(&dir, &files, Kind::TsJs, w);
    // The interface signature, the abstract one and the object-literal method; not the
    // `const` arrow function, a local to whoever reads `deleteUser` bare.
    assert_eq!(m("deleteUser"), [2, 9, 18]);
    assert_eq!(
        defs(&dir, &files, Kind::TsJs, "deleteUser"),
        [2, 9, 13, 18],
        "a bare word reads the `const` too"
    );
    // An optional generic signature with no `;`, not the call statement or the ternary.
    assert_eq!(m("findUser"), [3]);
    assert_eq!(m("onChange"), [4], "a property holding a function");
    assert_eq!(m("size"), [10], "a getter signature");
    // A plain field has no rule, and an annotated arrow argument is not a signature.
    assert_eq!(m("name"), Vec::<usize>::new());
    assert_eq!(m("run"), Vec::<usize>::new());
    // A call whose arguments hold `) :` or `):` is not one either; a signature whose
    // parameter is a function type, and a rest parameter, are.
    assert_eq!(m("append"), [26], "not the call on line 22");
    assert_eq!(m("log"), Vec::<usize>::new());
    assert_eq!(m("on"), [25]);
    // An empty body on the method's line, not a call whose last argument is one.
    assert_eq!(m("close"), [29]);
    assert_eq!(m("noop"), Vec::<usize>::new());
    // A constructor parameter behind an access modifier is a property of the class.
    assert_eq!(m("worker"), [34]);
    assert_eq!(m("inline"), [38]);
    assert_eq!(m("other"), [38]);
    // Type parameters may nest.
    assert_eq!(m("pong"), [41]);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A literal ends where the language ends it (#151): `"\\"` on its second quote, while a Rust
/// lifetime and a C++ digit separator open none, and a char literal still hides what it holds.
#[test]
fn a_literal_ends_where_the_language_ends_it() {
    assert_eq!(
        uncommented(Kind::Python, "sep = \"\\\\\"  # note"),
        "sep = \"\\\\\"  "
    );
    assert_eq!(
        returns(
            Kind::Python,
            "def windows_repo(sep=\"\\\\\") -> Repo:\n    return Repo()\n",
            1
        ),
        Some(ty("Repo"))
    );
    assert_eq!(
        uncommented(Kind::Rust, "fn f<'a>(x: &str) // note"),
        "fn f<'a>(x: &str) "
    );
    assert_eq!(
        uncommented(Kind::C, "int n = 1'000; // note"),
        "int n = 1'000; "
    );
    assert_eq!(
        uncommented(Kind::Rust, "let q = '\"'; // note"),
        "let q = '\"'; "
    );
    assert_eq!(close_of(Kind::Rust, "f('\\\\', ')')", 1), Some(12));
}

#[test]
fn lines_inside_a_literal_or_a_block_comment_are_told() {
    let inside = |kind, text: &str| -> Vec<usize> {
        let lines = literal_lines(kind, text);
        (1..=lines.len()).filter(|&n| lines[n - 1]).collect()
    };
    let py = "SRC = \"\"\"\ndef ghost(x):\n    pass\n\"\"\"\n\n\ndef real(a=\"# no\", b='\"\"\"'):  # it's fine\n    \"\"\"Doc.\n\n    def example():\n    \"\"\"\n    return 1\n";
    assert_eq!(inside(Kind::Python, py), [2, 3, 4, 9, 10, 11]);
    let go = "const s = `\nfunc (t T) InString() {}\n`\n\n/*\nfunc (t T) InBlock() {}\n*/\nfunc (t T) Real() { _ = \"/*\" } // it's `fine\nfunc (t T) Next() {}\n";
    assert_eq!(inside(Kind::Go, go), [2, 3, 6, 7]);
    let ts = "const q = `\n  find(id: string): User;\n  ${x}`;\nclass A {\n  find(id: string): User {}\n}\n";
    assert_eq!(inside(Kind::TsJs, ts), [2, 3]);
    // A migration embeds SQL, and a raw or a verbatim string is where it puts it. `""` is how
    // a verbatim string writes a quote, so it does not close one.
    let cs = "var q = \"\"\"\n    WHERE EXISTS(SELECT 1 FROM t)\n    \"\"\";\nvar v = @\"\n    SELECT MIN(\"\"rowid\"\") FROM t\n    \";\npublic int Real() => 1;\n";
    assert_eq!(inside(Kind::CSharp, cs), [2, 3, 5, 6]);
    let sw = "let doc = \"\"\"\n    class Ghost {}\n    \"\"\"\nclass Real {}\n";
    assert_eq!(inside(Kind::Swift, sw), [2, 3]);
    // A heredoc ends on the line that repeats its label, and only there.
    let php = "$sql = <<<SQL\n    function ghost() {}\n    class Ghost {}\nSQL;\n$n = <<<'TXT'\n    class Nowdoc {}\nTXT;\nclass Real {}\n";
    assert_eq!(inside(Kind::Php, php), [2, 3, 6]);
}

#[test]
fn a_go_signature_is_its_types_whatever_the_names() {
    let go = "type I interface {\n\tFerry(a, b string, ctx context.Context) (*Row, error)\n\tSolo(a string) error\n\tBare(string, int)\n}\nfunc (r *R) Ferry(x string, y string, c context.Context) (*db.Row, error) {\nfunc (n N) Solo(a int) string { return \"\" }\nfunc (n N) Bare(s string, i int) {}\nfunc (n N) Named(a int) (n int, err error) {\n";
    assert_eq!(go_signature(go, 2), go_signature(go, 6));
    assert!(go_signature(go, 2).is_some());
    assert_ne!(go_signature(go, 3), go_signature(go, 7));
    assert_eq!(go_signature(go, 4), go_signature(go, 8));
    assert_eq!(go_signature(go, 9), None);
}

#[test]
fn go_members_need_a_receiver() {
    let go = "package main\n\ntype Repo struct{}\n\nfunc (r *Repo[T]) Delete(id int) {}\n\nfunc (Repo) Find(id int) {}\n\nfunc Delete(id int) {}\n\nfunc main() {\n\tDelete := 1\n}\n\nfunc Get[T any](id int) {}\n";
    let (dir, files) = scratch("go-members", &[("repo.go", go)]);
    assert_eq!(
        defs(&dir, &files, Kind::Go, "Get"),
        [15],
        "a generic function"
    );
    assert_eq!(members(&dir, &files, Kind::Go, "Delete"), [5]);
    assert_eq!(members(&dir, &files, Kind::Go, "Find"), [7]);
    assert_eq!(defs(&dir, &files, Kind::Go, "Delete"), [5, 9, 12]);
    assert!(member_patterns(Kind::Rust, "len").is_none());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn qualified_names_come_from_the_declarations_around() {
    let py = "class Outer:\n    # a comment at the class level\n    class Inner:\n        def run(self):\n\n            pass\n\n    async def stop(self):\n        pass\n\ndef main():\n    def helper():\n        pass\n";
    let q = |kind, text, line, name| qualified(kind, text, line, name);
    assert_eq!(
        q(Kind::Python, py, 4, "run").as_deref(),
        Some("Outer.Inner.run")
    );
    assert_eq!(
        q(Kind::Python, py, 8, "stop").as_deref(),
        Some("Outer.stop")
    );
    assert_eq!(
        q(Kind::Python, py, 12, "helper").as_deref(),
        Some("main.helper")
    );
    assert_eq!(q(Kind::Python, py, 11, "main"), None, "the top level");
    assert_eq!(q(Kind::Python, py, 99, "x"), None);
    let ts = "export default class OrderService {\n  load(id: Id) {\n  }\n}\nexport interface Repo {\n  deleteUser(id: string): void;\n}\nexport const helpers = {\n  parse(s) {\n    return s;\n  },\n};\n";
    assert_eq!(
        q(Kind::TsJs, ts, 2, "load").as_deref(),
        Some("OrderService.load")
    );
    assert_eq!(
        q(Kind::TsJs, ts, 6, "deleteUser").as_deref(),
        Some("Repo.deleteUser")
    );
    assert_eq!(
        q(Kind::TsJs, ts, 9, "parse").as_deref(),
        Some("helpers.parse")
    );
    let go = "package main\n\nfunc (r *UserRepository) DeleteUser(id int) {}\nfunc (AuditLog) DeleteUser(id int) {}\nfunc (r *Repo[T]) Get() {}\nfunc Parse() {}\ntype Notifier interface {\n\tSend(text string)\n}\n";
    assert_eq!(
        q(Kind::Go, go, 3, "DeleteUser").as_deref(),
        Some("UserRepository.DeleteUser")
    );
    assert_eq!(
        q(Kind::Go, go, 4, "DeleteUser").as_deref(),
        Some("AuditLog.DeleteUser")
    );
    assert_eq!(q(Kind::Go, go, 5, "Get").as_deref(), Some("Repo.Get"));
    assert_eq!(q(Kind::Go, go, 6, "Parse"), None);
    assert_eq!(q(Kind::Go, go, 8, "Send").as_deref(), Some("Notifier.Send"));
    assert_eq!(q(Kind::Rust, RS, 6, "sum").as_deref(), Some("Order::sum"));
    // A Rust `impl` is named after the type it is for.
    let rs = "impl std::fmt::Display for Reason {\n    fn fmt(&self) {}\n}\nimpl<T> From<T> for Order {\n    fn from(t: T) -> Self {}\n}\n";
    assert_eq!(q(Kind::Rust, rs, 2, "fmt").as_deref(), Some("Reason::fmt"));
    assert_eq!(q(Kind::Rust, rs, 5, "from").as_deref(), Some("Order::from"));
    // An enclosing line that names nothing stops the walk: the object literal's `get` is
    // not `Api.get`.
    let lit = "class Api {\n  build() {\n    return {\n      get() {\n      },\n    };\n  }\n}\n";
    assert_eq!(q(Kind::TsJs, lit, 4, "get"), None);
    // A comment indented less than the method is skipped, not taken for a container.
    let commented = "class A:\n# note\n    def run(self):\n        pass\n";
    assert_eq!(
        q(Kind::Python, commented, 3, "run").as_deref(),
        Some("A.run")
    );
    let yaml = "x-common: &defaults\n  env: prod\n";
    assert_eq!(q(Kind::Yaml, yaml, 2, "env"), None);
    let rb = "module Billing\n  class Invoice\n    def total\n    end\n  end\nend\n";
    assert_eq!(
        q(Kind::Ruby, rb, 3, "total").as_deref(),
        Some("Billing.Invoice.total")
    );
}

/// #100. A `var (` block declares what stands at its own level; a function may declare a
/// name wherever it mentions it other than in front of a `.`.
#[test]
fn a_go_package_block_is_read_at_its_level_and_a_mention_may_declare() {
    let block = "var (\n\tcfg struct {\n\t\trepo *B\n\t}\n\n\t// the audit log\n\taudit AuditLog // shared\n)\n\nfunc f() {\n\trepo := 1\n}\n\ntype T struct {\n\taudit *B\n}\n";
    assert_eq!(package_bindings(block, "repo"), vec![]);
    assert_eq!(
        package_bindings(block, "audit"),
        vec![Binding {
            line: 7,
            value: Value::Type("AuditLog".into())
        }]
    );
    // The cursor is on the last `repo.Do()` of each.
    for (code, want) in [
        ("var x = repo.Do()\n", false),
        (
            "func Run(repo *A, fn func() error) {\n\trepo.Do()\n}\n",
            true,
        ),
        ("func Wide(\n\trepo *A,\n) error {\n\trepo.Do()\n}\n", true),
        (
            "func Struct(repo *A, fn func()) (out struct {\n\tX int\n}) {\n\trepo.Do()\n}\n",
            true,
        ),
        // A word of a comment or a string, a receiver of `.`, an argument handed on.
        (
            "func Free() {\n\t// repo is a word\n\tlog(\"repo\")\n\trepo.Do()\n\tsave(repo)\n\trepo.Do()\n}\n",
            false,
        ),
        ("func Arg() {\n\tsave(repo, 1)\n\trepo.Do()\n}\n", true),
        (
            "func Label() {\n\trepo := 1\nretry: // again\n\trepo.Do()\n}\n",
            true,
        ),
        // On the line itself.
        ("func One(repo *A, fn func()) { repo.Do() }\n", true),
        (
            "func If() {\n\tif repo := get(); repo.Do() {\n\t}\n}\n",
            true,
        ),
        // The function above is another function.
        (
            "func Above(repo *A) {\n}\n\nfunc Below() {\n\trepo.Do()\n}\n",
            false,
        ),
    ] {
        let text = format!("package p\n\n{code}");
        let lines: Vec<&str> = text.lines().collect();
        let line = lines.iter().rposition(|l| l.contains("repo.Do()")).unwrap() + 1;
        assert_eq!(go_may_declare(&text, line, "repo"), want, "{code}");
    }
}

/// #100. A Go package is the one directory its import path ends at.
#[test]
fn a_go_package_is_one_directory() {
    let parts = |p: &str| -> Vec<String> { p.split('/').map(str::to_owned).collect() };
    for (file, import, want) in [
        ("/go/src/database/sql/sql.go", "database/sql", true),
        (
            "/go/src/database/sql/driver/driver.go",
            "database/sql",
            false,
        ),
        (
            "/go/src/database/sql/driver/driver.go",
            "database/sql/driver",
            true,
        ),
        (
            "/go/src/vendor/golang.org/x/net/http2/h.go",
            "golang.org/x/net/http2",
            true,
        ),
        (
            "/mod/gopkg.in/yaml.v3@v3.0.1/yaml.go",
            "gopkg.in/yaml.v3",
            true,
        ),
        (
            "/mod/github.com/!burnt!sushi/toml@v1.2.3/lex.go",
            "github.com/BurntSushi/toml",
            true,
        ),
        (
            "/mod/github.com/foo/bar/v2@v2.1.0/sub/s.go",
            "github.com/foo/bar/v2/sub",
            true,
        ),
        (
            "/mod/github.com/foo/bar/v2@v2.1.0/sub/s.go",
            "github.com/foo/bar/v2",
            false,
        ),
        ("/go/src/errors/errors.go", "errors", true),
        (
            "/mod/github.com/pkg/errors@v0.9.1/errors.go",
            "errors",
            false,
        ),
        ("/go/src/internal/errors/e.go", "errors", false),
        ("/proj/vendor/errors/e.go", "errors", true),
        ("/proj/lib/errors/e.go", "errors", false),
    ] {
        assert_eq!(
            in_package(Path::new(file), &parts(import)),
            want,
            "{file} as {import}"
        );
    }
}

/// #100, #137. A Go file is compiled for a platform by its name and its `//go:build` line,
/// where a tag is set as a plain `go build` sets it: a tag of the project's own is not, until
/// `GOFLAGS` names it.
#[test]
fn a_go_file_is_built_for_a_platform_by_its_name_and_its_build_line() {
    let built = |name: &str, line: &str, os: &'static str, arch: &'static str| {
        let text = format!("// Copyright\n\n{line}\n\npackage p\n");
        let build = GoBuild {
            os,
            arch,
            cgo: true,
            tags: Vec::new(),
        };
        go_built(Path::new(name), &text, &build)
    };
    for (name, line, os, arch, want) in [
        ("clock.go", "", "linux", "amd64", Some(true)),
        ("clock_linux.go", "", "linux", "amd64", Some(true)),
        ("clock_linux.go", "", "darwin", "arm64", Some(false)),
        ("clock_linux_test.go", "", "darwin", "arm64", Some(false)),
        ("clock_arm64.go", "", "darwin", "arm64", Some(true)),
        ("clock_arm64.go", "", "darwin", "amd64", Some(false)),
        ("clock_linux_arm64.go", "", "linux", "arm64", Some(true)),
        ("clock_linux_arm64.go", "", "darwin", "arm64", Some(false)),
        // The name of a file is no ending of it, and `unix` is no `GOOS` a name can spell.
        ("linux.go", "", "darwin", "arm64", Some(true)),
        ("clock_unix.go", "", "windows", "amd64", Some(true)),
        (
            "clock_unix.go",
            "//go:build unix",
            "windows",
            "amd64",
            Some(false),
        ),
        (
            "clock_unix.go",
            "//go:build unix",
            "darwin",
            "arm64",
            Some(true),
        ),
        (
            "clock_other.go",
            "//go:build !windows",
            "linux",
            "amd64",
            Some(true),
        ),
        (
            "clock_other.go",
            "//go:build !windows",
            "windows",
            "amd64",
            Some(false),
        ),
        (
            "c.go",
            "//go:build linux || darwin",
            "darwin",
            "arm64",
            Some(true),
        ),
        (
            "c.go",
            "//go:build linux && arm64",
            "linux",
            "amd64",
            Some(false),
        ),
        (
            "c.go",
            "//go:build !(js && wasm)",
            "linux",
            "amd64",
            Some(true),
        ),
        (
            "c.go",
            "//go:build (linux || darwin) && !amd64",
            "darwin",
            "amd64",
            Some(false),
        ),
        (
            "c.go",
            "//go:build linux || darwin && amd64",
            "linux",
            "arm64",
            Some(true),
        ),
        // The name and the line both have to hold.
        (
            "c_linux.go",
            "//go:build arm64",
            "linux",
            "amd64",
            Some(false),
        ),
        ("c.go", "//go:build gogit", "linux", "amd64", Some(false)),
        (
            "c.go",
            "//go:build !gogit && linux",
            "linux",
            "amd64",
            Some(true),
        ),
        (
            "c.go",
            "//go:build !gogit && linux",
            "darwin",
            "arm64",
            Some(false),
        ),
        ("c.go", "//go:build ignore", "linux", "amd64", Some(false)),
        // What every toolchain sets: cgo, the compiler, the releases behind it.
        ("c.go", "//go:build go1.21", "linux", "amd64", Some(true)),
        ("c.go", "//go:build !go1.21", "linux", "amd64", Some(false)),
        ("c.go", "//go:build gc", "linux", "amd64", Some(true)),
        ("c.go", "//go:build gccgo", "linux", "amd64", Some(false)),
        (
            "c.go",
            "//go:build windows && cgo",
            "darwin",
            "arm64",
            Some(false),
        ),
        (
            "c.go",
            "//go:build cgo && windows",
            "darwin",
            "arm64",
            Some(false),
        ),
        (
            "c.go",
            "//go:build darwin || cgo",
            "darwin",
            "arm64",
            Some(true),
        ),
        (
            "c.go",
            "//go:build darwin && cgo",
            "darwin",
            "arm64",
            Some(true),
        ),
        (
            "c.go",
            "//go:build windows || cgo",
            "darwin",
            "arm64",
            Some(true),
        ),
        (
            "c.go",
            "//go:build !(windows && cgo)",
            "darwin",
            "arm64",
            Some(true),
        ),
        // The old spelling is not read, nor is a line that does not parse; an architecture
        // is one whatever the host.
        ("c.go", "// +build windows", "darwin", "arm64", None),
        ("c.go", "//go:build darwin &&", "darwin", "arm64", None),
        ("c_sparc64.go", "", "darwin", "arm64", Some(false)),
    ] {
        assert_eq!(
            built(name, line, os, arch),
            want,
            "{name} {line} on {os}/{arch}"
        );
    }
    // The host is spelled as Go spells it.
    let host = GoBuild::host();
    let (os, arch) = (host.os, host.arch);
    assert!(GO_UNIX.contains(&os) || GO_OS.contains(&os), "{os}");
    assert!(GO_ARCH.contains(&arch), "{arch}");
    // The environment is read as `go build` reads it: the last `-tags` of `GOFLAGS`.
    let env = host.env(Some("0"), Some("-mod=mod -tags=old --tags=gogit,bindata"));
    let tagged = |line: &str| go_built(Path::new("c.go"), &format!("{line}\npackage p\n"), &env);
    assert_eq!(tagged("//go:build gogit"), Some(true));
    assert_eq!(tagged("//go:build !bindata"), Some(false));
    assert_eq!(tagged("//go:build old || cgo"), Some(false));
    assert!(GoBuild::host().env(Some("1"), None).cgo);
    let linux = GoBuild {
        os: "linux",
        arch: "amd64",
        ..GoBuild::host()
    };
    // So is one inside a block comment above it.
    let block = "/*\n//go:build windows\n*/\n\npackage p\n";
    assert_eq!(go_built(Path::new("c.go"), block, &linux), Some(true));
    // A `//go:build` under the package clause is a comment.
    let late = "package p\n\n//go:build windows\n";
    assert_eq!(go_built(Path::new("c.go"), late, &linux), Some(true));
}

/// #104. What the search by name offers for a field — the lines [`field_patterns`] match,
/// through [`field_rows`] — is each type's declaration of it once, named after the type and
/// not after the method it is assigned in. A later assignment stands for the declaration; a
/// local, the key of a literal, a docstring, a `var` block, a field of an anonymous struct, a
/// member of a type literal and a `this` that is no class are none.
#[test]
fn a_field_by_name_is_the_declaration_in_its_type() {
    let found = |kind: Kind, text: &str, name: &str| -> Vec<(usize, String)> {
        let re = Regex::new(&field_patterns(kind, name).unwrap().join("|")).unwrap();
        let hits: Vec<usize> = text
            .lines()
            .enumerate()
            .filter(|(_, l)| re.is_match(l))
            .map(|(i, _)| i + 1)
            .collect();
        field_rows(kind, text, &hits, name)
            .into_iter()
            .map(|n| (n, qualified(kind, text, n, name).unwrap_or_default()))
            .collect()
    };
    let one = |n: usize, q: &str| vec![(n, q.to_owned())];
    let py = "class Issue(Base):\n    poster_id: int = 0\n\n    def __init__(\n        self,\n        repo: Repo,\n    ) -> None:\n        self.repo = repo\n        self.poster_id = 1\n\n    def close(self) -> None:\n        self.repo = None\n        total: int = 0\n        counts = {\n            total: 1,\n        }\n\n\ndef tally() -> None:\n    repo: Repo = make()\n";
    assert_eq!(
        found(Kind::Python, py, "poster_id"),
        one(2, "Issue.poster_id")
    );
    assert_eq!(found(Kind::Python, py, "repo"), one(8, "Issue.repo"));
    assert_eq!(found(Kind::Python, py, "total"), vec![]);
    assert_eq!(
        qualified(Kind::Python, py, 12, "repo").as_deref(),
        Some("Issue.repo")
    );
    // Outside a class `self` is a parameter like any other.
    let def = "def build(self):\n    self.x = 1\n";
    assert_eq!(found(Kind::Python, def, "x"), vec![]);
    assert_eq!(
        qualified(Kind::Python, def, 2, "x").as_deref(),
        Some("build.x")
    );
    // A tuple target is a declaration, and the later plain assignment stands for it.
    let tuple = "class Point:\n    def __init__(self):\n        self.x, self.offset = 0, 0\n\n    def move(self):\n        self.offset = 5\n";
    assert_eq!(found(Kind::Python, tuple, "offset"), one(3, "Point.offset"));
    let doc = "class Comment:\n    \"\"\"\n    body : str\n    \"\"\"\n\n    def __init__(self, body):\n        self.body = body\n";
    assert_eq!(found(Kind::Python, doc, "body"), one(7, "Comment.body"));
    let ts = "export class Issue extends Base {\n  posterId = 0;\n\n  constructor(\n    private repo: Repo,\n    @Inject(Log) private log: Log,\n  ) {\n    super();\n    this.title = \"\";\n  }\n\n  resize(opts: {\n    readonly width: number;\n  }): void {\n    const box = {\n      grow() {\n        this.height = 1;\n      },\n    };\n  }\n}\nexport function tally(): void {\n  const sums = {\n    total: 0,\n  };\n  total = 2;\n}\nexport const config: {\n  readonly timeout: number;\n} = { timeout: 1 };\n";
    assert_eq!(found(Kind::TsJs, ts, "posterId"), one(2, "Issue.posterId"));
    assert_eq!(found(Kind::TsJs, ts, "repo"), one(5, "Issue.repo"));
    assert_eq!(found(Kind::TsJs, ts, "log"), one(6, "Issue.log"));
    assert_eq!(found(Kind::TsJs, ts, "title"), one(9, "Issue.title"));
    assert_eq!(found(Kind::TsJs, ts, "width"), vec![]);
    assert_eq!(found(Kind::TsJs, ts, "height"), vec![]);
    // Nor are they named after the class.
    assert_eq!(qualified(Kind::TsJs, ts, 13, "width"), None);
    assert_eq!(qualified(Kind::TsJs, ts, 17, "height"), None);
    assert_eq!(found(Kind::TsJs, ts, "total"), vec![]);
    assert_eq!(found(Kind::TsJs, ts, "timeout"), vec![]);
    assert_eq!(
        qualified(Kind::TsJs, ts, 29, "timeout").as_deref(),
        Some("config.timeout")
    );
    let go = "package main\n\ntype Issue struct {\n\t*store.Base\n\tPosterID    int\n\tTitle, Body string `json:\"t\"`\n\tStats       struct {\n\t\tTotal int\n\t}\n}\n\nfunc Serve() {\n\tvar (\n\t\tHost string\n\t)\n}\n";
    assert_eq!(found(Kind::Go, go, "PosterID"), one(5, "Issue.PosterID"));
    assert_eq!(found(Kind::Go, go, "Body"), one(6, "Issue.Body"));
    assert_eq!(found(Kind::Go, go, "Base"), one(4, "Issue.Base"));
    assert_eq!(found(Kind::Go, go, "Stats"), one(7, "Issue.Stats"));
    assert_eq!(found(Kind::Go, go, "Total"), vec![]);
    assert_eq!(found(Kind::Go, go, "Host"), vec![]);
}

/// #104. The search by name greps for [`member_patterns`] and [`field_patterns`] and then keeps
/// the fields [`field_bindings`] reads; a form the grep does not know would drop its type from
/// the list, so every line the rules read must be one the grep finds.
#[test]
fn the_grep_finds_every_line_the_field_rules_read() {
    let py = "class A:\n    a: int\n    b = 1\n\n    def __init__(self):\n        self.c = 1\n        self.d: int = 1\n        self.e, self.f = 1, 2\n        (self.g, x) = 1, 2\n        with open(p) as self.h:\n            pass\n        for self.i in xs:\n            pass\n        if p: self.k = 1\n\n    if p: m: int = 1\n\n    @property\n    def j(self) -> int:\n        return 1\n";
    let ts = "export class A {\n  a: number;\n  b = 1;\n  static c = 1;\n  declare d: D;\n  accessor e = 1;\n  f;\n  readonly g?: G;\n  get h(): H {\n    return new H();\n  }\n\n  constructor(\n    private i: I,\n    @Inject(J) protected j: J,\n  ) {\n    this.k = 1;\n  }\n}\nexport class B {\n  constructor(private l: L) {}\n}\n";
    let go = "package main\n\ntype A struct {\n\tA int\n\tB, C string\n\t*Base\n\tpkg.Mixin\n\tD func(x int) error\n\tE map[string]int `json:\"e\"`\n}\n";
    let cases: [(Kind, &str, &[&str]); 3] = [
        (
            Kind::Python,
            py,
            &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "m"],
        ),
        (
            Kind::TsJs,
            ts,
            &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"],
        ),
        (Kind::Go, go, &["A", "B", "C", "Base", "Mixin", "D", "E"]),
    ];
    for (kind, text, names) in cases {
        let lines: Vec<&str> = text.lines().collect();
        let decls: Vec<usize> = (1..=lines.len())
            .filter(|&n| declares_type(kind, lines[n - 1]))
            .collect();
        for name in names {
            let mut patterns = member_patterns(kind, name).unwrap();
            patterns.extend(field_patterns(kind, name).unwrap());
            let re = Regex::new(&patterns.join("|")).unwrap();
            let read: Vec<usize> = decls
                .iter()
                .flat_map(|&d| field_bindings(kind, text, d, name))
                .map(|b| b.line)
                .collect();
            assert!(!read.is_empty(), "{kind:?}: no field {name}");
            for n in read {
                assert!(
                    re.is_match(lines[n - 1]),
                    "{kind:?}: {name}: {}",
                    lines[n - 1]
                );
            }
        }
    }
}

#[test]
fn a_reason_says_whether_it_proves_the_target() {
    assert_eq!(Reason::ByName.to_string(), "by name");
    assert!(!Reason::ByName.proven());
    let import = Reason::Import("numpy".into());
    assert_eq!(import.to_string(), "via import numpy");
    assert!(import.proven());
    assert_eq!(Reason::Path("std::fs".into()).to_string(), "via std::fs");
}

#[test]
fn go_def_patterns_cover_receivers_types_and_short_vars() {
    let (dir, files) = project("go");
    let go = files[1..2].to_vec();
    for (word, line) in [("Invoice", 3), ("Total", 5), ("Parse", 7), ("Limit", 9)] {
        assert_eq!(defs(&dir, &go, Kind::Go, word), [line], "{word}");
    }
    // `inv := Parse("x")` is the closest thing Go has to a definition of `inv`.
    assert_eq!(defs(&dir, &go, Kind::Go, "inv"), [12]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rust_def_patterns_cover_items_behind_prefixes_and_lets() {
    let (dir, files) = project("rs");
    let rs = files[2..3].to_vec();
    for (word, line) in [
        ("Order", 1), // the struct, not the `impl` block or the `Order { .. }` literal
        ("sum", 6),
        ("MAX_ORDERS", 9),
        ("parse_order", 11),
        ("orders", 21),
    ] {
        assert_eq!(defs(&dir, &rs, Kind::Rust, word), [line], "{word}");
    }
    // Both the `let mut` binding and the macro: the caller shows a picker.
    assert_eq!(defs(&dir, &rs, Kind::Rust, "order"), [12, 17]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn ts_def_patterns_cover_declarations_methods_and_arrows() {
    let (dir, files) = project("ts");
    // `.ts` and `.js` are one kind: the JS helpers are found from the TS file.
    let ts_js = files[3..].to_vec();
    for (word, file, line) in [
        ("Order", "d.ts", 1),
        ("Id", "d.ts", 5),
        ("Status", "d.ts", 7),
        ("OrderService", "d.ts", 11),
        ("load", "d.ts", 14),
        ("sum", "d.ts", 19),
        ("parseOrder", "d.ts", 22),
        ("render", "d.ts", 24), // the declaration, not the `render(o);` call
        ("n", "d.ts", 25),
        ("ids", "d.ts", 28),
        ("parse", "e.js", 2),
        ("format", "e.js", 5),
    ] {
        let pat = def_patterns(Kind::TsJs, word).join("|");
        assert_eq!(
            lines(&grep(&dir, &ts_js, &pat, false, false)),
            [(file.into(), line)],
            "{word}"
        );
    }
    // A plain field is not a declaration the rules know: `d` has no definition for it.
    assert!(defs(&dir, &ts_js, Kind::TsJs, "cache").is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

const JAVA: &str = r#"package app;

public final class Invoice {
    private static final int LIMIT = 10;
    private Map<String, Integer> items;

    public Invoice(int n) {
        this.n = n;
    }

    @Override
    public int total() {
        return compute(items);
    }

    static Map<String, Integer> compute(Map<String, Integer> rows) {
        return rows;
    }

    Runnable onSave() {
        return new Runnable() {
            public void run() {}
        };
    }
}

interface Store {
    void save(Invoice inv);
}

record Point(int x, int y) {}

enum Status {
    OPEN,
}
"#;

const KT: &str = r#"package app

data class Order(val id: String)

class Service(private val repo: Repo) {
    private val cache = mutableMapOf<String, Order>()

    suspend fun load(id: String): Order {
        return parse(id)
    }

    fun parse(id: String): Order = Order(id)
}

fun String.slug(): String = lowercase()

fun Card(title: String, content: () -> Unit) {
}

fun screen() {
    Card(title = "Invoice") {
        withContext(Dispatchers.IO) {
        }
    }
    Card(title = "Order") {
    }
}

object Registry {
    val all = listOf<Order>()

    private const val TAG = "Invoice"
}

enum class Status {
    OPEN,
}

typealias Rows = List<Order>

const val LIMIT = 10

fun interface Handler {
    fun handle(order: Order)
}
"#;

#[test]
fn java_def_patterns_cover_types_methods_and_fields() {
    let (dir, files) = scratch("java", &[("Invoice.java", JAVA)]);
    let d = |w| defs(&dir, &files, Kind::Jvm, w);
    // The class and the constructor; the caller shows a picker.
    assert_eq!(d("Invoice"), [3, 7]);
    assert_eq!(d("LIMIT"), [4]);
    assert_eq!(d("items"), [5], "not the `compute(items)` call");
    assert_eq!(d("total"), [12], "behind its annotation and modifiers");
    // The declaration, not the `return compute(items);` call above it.
    assert_eq!(d("compute"), [16]);
    assert_eq!(d("onSave"), [20], "no modifier, but a return type");
    assert_eq!(d("run"), [22]);
    assert_eq!(d("Store"), [27]);
    assert_eq!(d("save"), [28], "an interface method has no body");
    assert_eq!(d("Point"), [31]);
    assert_eq!(d("Status"), [33]);
    assert_eq!(d("rows"), Vec::<usize>::new(), "a parameter has no rule");
    // `new Runnable() {` opens an anonymous class: a use of the interface, not a
    // declaration of it.
    assert_eq!(d("Runnable"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn kotlin_def_patterns_cover_declarations_and_receivers() {
    let (dir, files) = scratch("kt", &[("app.kt", KT)]);
    let d = |w| defs(&dir, &files, Kind::Jvm, w);
    assert_eq!(d("Order"), [3], "not the `Order(id)` calls");
    assert_eq!(d("Service"), [5]);
    assert_eq!(d("cache"), [6]);
    assert_eq!(d("load"), [8], "behind `suspend`");
    assert_eq!(d("parse"), [12], "not the `return parse(id)` call");
    assert_eq!(d("slug"), [15], "an extension function, past its receiver");
    assert_eq!(d("screen"), [20]);
    // The `fun`, not the two `Card(title = …) {` calls with a trailing lambda.
    assert_eq!(d("Card"), [17]);
    assert_eq!(
        d("withContext"),
        Vec::<usize>::new(),
        "a call with a trailing lambda is not a declaration"
    );
    assert_eq!(d("Registry"), [29]);
    assert_eq!(d("all"), [30]);
    assert_eq!(d("TAG"), [32], "behind `private const`");
    assert_eq!(d("Status"), [35]);
    assert_eq!(d("Rows"), [39]);
    assert_eq!(d("LIMIT"), [41]);
    assert_eq!(d("Handler"), [43], "a `fun interface`");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn java_and_kotlin_find_each_other() {
    let (dir, files) = scratch("jvm", &[("Invoice.java", JAVA), ("app.kt", KT)]);
    let pat = def_patterns(Kind::Jvm, "Store").join("|");
    assert_eq!(
        lines(&grep(&dir, &files, &pat, false, false)),
        [("Invoice.java".into(), 27)]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const RB: &str = r#"module Billing
  LIMIT = 10

  class Invoice
    attr_accessor :total
    attr_reader :id, :customer

    @@count = 0

    def initialize(id)
      @id = id
      @rows = []
    end

    def self.parse(text)
      new(text)
    end

    def total=(value)
      @total = value
    end

    def cache
      @cache ||= {}
    end

    def empty?
      @rows.empty?
    end

    def ==(other)
      total == other.total && name =~ /x/
    end

    def to_h
      {
        id => 1,
      }
    end

    alias_method :blank?, :empty?
  end
end
"#;

#[test]
fn ruby_def_patterns_find_methods_attributes_and_assignments() {
    let (dir, files) = scratch("rb", &[("invoice.rb", RB)]);
    let d = |w| defs(&dir, &files, Kind::Ruby, w);
    assert_eq!(d("Billing"), [1]);
    assert_eq!(d("LIMIT"), [2], "a constant, indented in its module");
    assert_eq!(d("Invoice"), [4]);
    // The accessor, the setter and the assignment behind it -- not `total == other.total`.
    assert_eq!(d("total"), [5, 19, 20]);
    assert_eq!(d("customer"), [6], "second in the `attr_reader` list");
    // `id => 1,` is a hash pair, not an assignment.
    assert_eq!(d("id"), [6, 11]);
    assert_eq!(d("count"), [8], "a class variable");
    assert_eq!(d("initialize"), [10]);
    assert_eq!(d("parse"), [15], "`def self.parse`");
    assert_eq!(
        d("cache"),
        [23, 24],
        "the method and the `||=` it memoises with"
    );
    // `?` is not part of the word under the cursor, and `@rows.empty?` is a call.
    assert_eq!(d("empty"), [27]);
    assert_eq!(d("rows"), [12]);
    assert_eq!(d("blank"), [41]);
    assert_eq!(d("name"), Vec::<usize>::new(), "`name =~ /x/` is a match");
    assert_eq!(d("new"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

const C_H: &str = r#"#ifndef INVOICE_H
#define INVOICE_H

#define LRU_BITS 24
#define serverLog(level, ...) do { emit(level); } while (0)

typedef char *sds;
typedef int (*compare_fn)(const void *a, const void *b);

struct client;

struct __attribute__ ((__packed__)) sdshdr8 {
    uint8_t len;
};

static struct config {
    int port;
} server_config;

typedef struct invoice {
    sds name;
    int total;
    struct {
        int index;
    } offset;
} invoice;

typedef enum {
    STATE_NONE = 0,
    STATE_OPEN,
} state;

union value {
    int n;
    sds s;
};

extern struct invoice *current;

int invoice_total(struct invoice *inv);

#endif
"#;

const C_C: &str = r#"#include "invoice.h"

struct invoice *current = NULL;
static int counter;

int invoice_total(struct invoice *inv) {
    if (invoice_valid(inv)) {
        return compute(inv);
    }
    return counter;
}

static int compute(struct invoice *inv)
{
    struct invoice *copy = inv;
    return copy->total;
}

static unsigned
invoice_index(const struct invoice *inv)
{
    return 0;
}

void invoice_free(struct invoice *inv,
                  int deep)
{
    free(inv);
}
"#;

const CPP: &str = r#"#include "invoice.h"

namespace billing {

using Rows = std::vector<int>;
using std::swap;

template <typename T> struct Box : Base {
  T value;
};

template <typename T> struct Box<T *> : Base {
};

template <typename T = int>
class LEDGER_API Ledger : public Base {
 public:
  explicit Ledger(int n) : total_(n) {}

  auto total() const -> int { return total_; }

  void append(Rows rows);

  using Row = int;

 private:
  int total_;
};

void Ledger::append(Rows rows) {
  if (check(rows)) {
    log::write(rows);
  }
}

enum class Status {
  Open,
};

}  // namespace billing
"#;

#[test]
fn c_def_patterns_find_types_macros_functions_and_globals() {
    let (dir, files) = scratch("c-h", &[("invoice.h", C_H)]);
    let d = |w| defs(&dir, &files, Kind::C, w);
    assert_eq!(d("LRU_BITS"), [4]);
    assert_eq!(d("serverLog"), [5], "a function-like macro");
    assert_eq!(d("sds"), [7], "not the `sds name;` field it types");
    assert_eq!(d("compare_fn"), [8], "a function pointer");
    assert_eq!(d("client"), [10], "a forward declaration");
    assert_eq!(d("sdshdr8"), [12], "behind a lower-case attribute");
    assert_eq!(d("config"), [16], "behind a storage specifier");
    assert_eq!(d("server_config"), [18], "the name the block closes with");
    // The `typedef struct invoice {` and the `} invoice;` it closes with: `d` offers both,
    // where `D` lists the type once, under the name the project uses.
    assert_eq!(d("invoice"), [20, 26]);
    assert_eq!(d("state"), [31]);
    assert_eq!(d("value"), [33]);
    assert_eq!(d("current"), [38]);
    assert_eq!(d("invoice_total"), [40], "the prototype");
    assert_eq!(d("total"), Vec::<usize>::new(), "a field has no rule");
    assert_eq!(
        d("offset"),
        Vec::<usize>::new(),
        "an indented closing brace ends a nested anonymous struct: a field, not a type"
    );
    assert_eq!(
        d("STATE_OPEN"),
        Vec::<usize>::new(),
        "an enum constant has no rule: `NAME,` is also a line of an initializer list"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn c_def_patterns_tell_a_definition_from_a_call() {
    let (dir, files) = scratch("c-c", &[("invoice.c", C_C)]);
    let d = |w| defs(&dir, &files, Kind::C, w);
    assert_eq!(d("current"), [3]);
    assert_eq!(d("counter"), [4], "a global, not the `return counter;`");
    assert_eq!(d("invoice_total"), [6]);
    assert_eq!(d("compute"), [13], "the brace opens on the next line");
    assert_eq!(
        d("invoice_index"),
        [20],
        "the return type is on the line above"
    );
    assert_eq!(d("invoice_free"), [25], "the parameters wrap");
    // Column zero is where C declares; indented, only a body opening on the line counts.
    assert_eq!(
        d("invoice_valid"),
        Vec::<usize>::new(),
        "a call inside `if`"
    );
    assert_eq!(d("free"), Vec::<usize>::new(), "a call statement");
    assert_eq!(d("copy"), Vec::<usize>::new(), "a local");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn cpp_def_patterns_cover_classes_methods_and_aliases() {
    let (dir, files) = scratch("cpp", &[("ledger.cc", CPP)]);
    let d = |w| defs(&dir, &files, Kind::C, w);
    assert_eq!(d("billing"), [3], "not the closing comment");
    assert_eq!(d("Rows"), [5]);
    assert_eq!(d("Box"), [8, 12], "the template and its specialization");
    assert_eq!(
        d("Ledger"),
        [16, 18],
        "past the template head; and its constructor"
    );
    assert_eq!(d("total"), [20], "a method defined in the class body");
    assert_eq!(d("Row"), [24], "a `using` alias indented in a class body");
    // The out-of-line definition. The declaration on line 22 has no rule: indented, it is
    // the shape of a call, and the definition is what `d` is asked for anyway.
    assert_eq!(d("append"), [30]);
    assert_eq!(d("Status"), [36]);
    assert_eq!(
        d("T"),
        Vec::<usize>::new(),
        "a template parameter is no global: `template <typename T = int>` declares nothing"
    );
    assert_eq!(
        d("swap"),
        Vec::<usize>::new(),
        "`using std::swap;` imports a name"
    );
    assert_eq!(d("check"), Vec::<usize>::new(), "a call inside `if`");
    assert_eq!(d("write"), Vec::<usize>::new(), "a qualified call");
    assert_eq!(d("Open"), Vec::<usize>::new(), "an enum constant");
    assert_eq!(d("total_"), Vec::<usize>::new(), "a field");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn c_and_cpp_scope_roots_and_names() {
    // One kind: a `.cc` is searched for a definition asked for in a `.h`, and a file of
    // another kind is not.
    let here = Path::new("ledger.hpp");
    assert!(in_def_scope(Kind::C, here, Path::new("src/invoice.c")));
    assert!(in_def_scope(Kind::C, here, Path::new("ledger.cc")));
    assert!(!in_def_scope(Kind::C, here, Path::new("main.rs")));
    // `#include` binds no name, and a member of a value has no rule of its own.
    assert!(imports(Kind::C, C_C).is_empty());
    assert!(member_patterns(Kind::C, "total").is_none());
    // The system headers, wherever this machine keeps them: the SDK on a Mac,
    // `/usr/include` on Linux. Both exist on CI, so the list is never empty there.
    let roots = external_roots(Kind::C, Path::new("/"));
    assert!(
        roots.iter().any(|r| r.ends_with("usr/include")),
        "no system include directory among {roots:?}"
    );
    // C++ writes a member behind `::`, as Rust does, and an access specifier is a label
    // inside the class, not a wall in front of it. The enclosing `namespace billing {` is
    // not indented, so, as in every kind, the walk ends at the first column-zero declaration.
    assert_eq!(
        qualified(Kind::C, CPP, 20, "total").as_deref(),
        Some("Ledger::total")
    );
    assert_eq!(qualified(Kind::C, CPP, 3, "billing"), None);
}

#[test]
fn c_and_cpp_files_find_each_other() {
    let (dir, files) = scratch(
        "c-family",
        &[("invoice.h", C_H), ("invoice.c", C_C), ("ledger.cc", CPP)],
    );
    // The header's prototype and the definition that follows it are both offered; the
    // picker's rows say which file each is in.
    let pat = def_patterns(Kind::C, "invoice_total").join("|");
    assert_eq!(
        lines(&grep(&dir, &files, &pat, false, false)),
        [("invoice.c".into(), 6), ("invoice.h".into(), 40)],
        "the picker's order is by path, as everywhere: it is not definition before prototype"
    );
    // A type of the C header, reached from the C++ file that includes it.
    let pat = def_patterns(Kind::C, "sds").join("|");
    assert_eq!(
        lines(&grep(&dir, &files, &pat, false, false)),
        [("invoice.h".into(), 7)]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

const CS: &str = r#"namespace Billing.Core;

using Rows = System.Collections.Generic.List<int>;

[Serializable]
public sealed partial class Invoice<T> : Base, IEnumerable<T>
{
    private const int Limit = 10;
    private readonly ILogger<Invoice<T>> _logger;
    public static event EventHandler? Saved;

    public Invoice(int n)
    {
        _logger = Create(n);
    }

    public int Total { get; private set; }

    public string Name => _name;

    [HttpGet("{id}")]
    public async Task<Invoice<T>> LoadAsync(int id)
    {
        var rows = Compute(id);
        if (Check(rows))
        {
            Console.WriteLine(rows);
        }
        return new Invoice<T>(id);
    }

    int IComparable.CompareTo(object? other) => 0;

    private static Rows Compute(int id) => new Rows();
}

public interface IStore
{
    void Save(Invoice<int> inv);
}

public record struct Point(int X, int Y);

public record Money(decimal Amount);

public enum Status
{
    Open,
}

public delegate int Comparison<T>(T a, T b);

public static class Registry
{
    public static Dictionary<string, Invoice<int>> All = new();
}
"#;

#[test]
fn csharp_def_patterns_find_types_members_and_fields() {
    let (dir, files) = scratch("cs", &[("Invoice.cs", CS)]);
    let d = |w| defs(&dir, &files, Kind::CSharp, w);
    assert_eq!(d("Core"), [1], "a file-scoped namespace, by its last part");
    assert_eq!(
        d("Rows"),
        [3],
        "a `using` alias, not the `Rows` it is used as"
    );
    // The class past its generic parameters and its attribute, and the constructor; the
    // caller shows a picker.
    assert_eq!(d("Invoice"), [6, 12]);
    assert_eq!(d("Limit"), [8]);
    assert_eq!(d("_logger"), [9], "not the `_logger = Create(n);` write");
    assert_eq!(d("Saved"), [10], "an event");
    assert_eq!(d("Total"), [17], "a property, by its accessor block");
    assert_eq!(d("Name"), [19], "an expression-bodied property");
    assert_eq!(
        d("LoadAsync"),
        [22],
        "behind an attribute and `public async`"
    );
    assert_eq!(d("rows"), [24], "a local");
    assert_eq!(d("Compute"), [34], "not the `Compute(id)` call above it");
    assert_eq!(d("CompareTo"), [32], "an explicit interface implementation");
    assert_eq!(d("IStore"), [37]);
    assert_eq!(d("Save"), [39], "an interface method has no modifiers");
    assert_eq!(d("Point"), [42], "a positional `record struct`");
    assert_eq!(d("Money"), [44]);
    assert_eq!(d("Status"), [46]);
    assert_eq!(d("Comparison"), [51], "a delegate, past its return type");
    assert_eq!(d("Registry"), [53]);
    assert_eq!(d("All"), [55]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn csharp_def_patterns_tell_a_declaration_from_a_call() {
    let (dir, files) = scratch("cs-calls", &[("Invoice.cs", CS)]);
    let d = |w| defs(&dir, &files, Kind::CSharp, w);
    let none = Vec::<usize>::new();
    assert_eq!(d("Check"), none, "a call inside `if`");
    assert_eq!(d("Console"), none);
    assert_eq!(d("WriteLine"), none, "a call statement");
    assert_eq!(d("Create"), none, "a call on the right of an assignment");
    assert_eq!(d("Base"), none, "a base list is a use of the type");
    assert_eq!(d("IEnumerable"), none);
    assert_eq!(d("id"), none, "a parameter has no rule");
    assert_eq!(
        d("Open"),
        none,
        "an enum member has no rule: `Open,` is also a line of a collection initialiser"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn csharp_scope_stays_in_the_project() {
    let here = Path::new("Invoice.cs");
    assert!(in_def_scope(Kind::CSharp, here, Path::new("src/Store.csx")));
    assert!(!in_def_scope(Kind::CSharp, here, Path::new("main.rs")));
    // A `using` opens a whole namespace, so it binds no name of its own, and there is nothing
    // to bind it to: a NuGet package ships assemblies, not source.
    assert!(imports(Kind::CSharp, CS).is_empty());
    assert!(external_roots(Kind::CSharp, Path::new("/")).is_empty());
    assert!(member_patterns(Kind::CSharp, "Total").is_none());
    // A member is named by the type it is declared in, as the picker rows show it.
    assert_eq!(
        qualified(Kind::CSharp, CS, 22, "LoadAsync").as_deref(),
        Some("Invoice.LoadAsync")
    );
}

const SWIFT: &str = r#"import Foundation

public protocol RequestDelegate: AnyObject {
    associatedtype Value

    func didFinish(_ request: Request)
}

@objc(AFSession)
public final class Session: NSObject {
    public static let `default` = Session()

    private let queue: DispatchQueue
    public var isRunning = false

    public init(queue: DispatchQueue = .main) {
        self.queue = queue
    }

    convenience init?(name: String) {
        self.init()
    }

    public func request<T: Encodable>(_ url: URL, with body: T) -> Request {
        let request = Request(url)
        queue.async {
            self.start(request)
        }
        if let delegate = delegate {
            delegate.didFinish(request)
        }
        return request
    }

    class func shared() -> Session {
        return Session()
    }
}

extension Session: RequestDelegate {
    public func didFinish(_ request: Request) {
        switch request.state {
        case .finished:
            break
        case let .failed(error):
            print(error)
        }
    }
}

public enum State {
    case initialized
    case resumed(Int), suspended
    case failed(Error)
}

public struct Response<Value> {
    let value: Value
}

actor Cache {
    var entries: [String: Data] = [:]
}

public typealias Rows = [Int]

public final class Store {
    public private(set) weak var owner: Session?
}

let opened = 0

func describe(_ code: Int) -> String {
    switch code {
    case opened:
        return "opened"
    default:
        return ""
    }
}
"#;

#[test]
fn swift_def_patterns_find_declarations_behind_attributes_and_modifiers() {
    let (dir, files) = scratch("swift", &[("Session.swift", SWIFT)]);
    let d = |w| defs(&dir, &files, Kind::Swift, w);
    assert_eq!(d("RequestDelegate"), [3], "a protocol");
    assert_eq!(d("Value"), [4], "an `associatedtype`");
    // The class and the extension of it: a project's own members of a type live in one.
    assert_eq!(d("Session"), [10, 40], "not the `Session()` calls");
    assert_eq!(d("didFinish"), [6, 41], "not the `delegate.didFinish` call");
    assert_eq!(d("default"), [11], "a backticked name");
    assert_eq!(d("queue"), [13], "not the `self.queue = queue` write");
    assert_eq!(d("isRunning"), [14]);
    assert_eq!(
        d("init"),
        [16, 20],
        "`init?` too, not the `self.init()` call"
    );
    assert_eq!(d("request"), [24, 25], "the function and the local");
    assert_eq!(d("shared"), [35], "behind `class`, Swift's static method");
    assert_eq!(d("initialized"), [52], "an enum case");
    assert_eq!(d("resumed"), [53], "with its associated value");
    assert_eq!(d("suspended"), [53], "second on the line");
    assert_eq!(
        d("failed"),
        [54],
        "not the `case let .failed(error):` pattern"
    );
    assert_eq!(d("State"), [51]);
    assert_eq!(d("Response"), [57], "past the generic parameters");
    assert_eq!(d("Cache"), [61], "an actor");
    assert_eq!(d("entries"), [62]);
    assert_eq!(d("Rows"), [65], "a `typealias`");
    assert_eq!(d("Store"), [67]);
    assert_eq!(d("owner"), [68], "behind `public private(set) weak`");
    // The `let`, not the `case opened:` of the `switch` below it, which matches against
    // that constant: a bare name there is a pattern, not a declaration.
    assert_eq!(d("opened"), [71]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn swift_def_patterns_tell_a_declaration_from_a_call_or_a_pattern() {
    let (dir, files) = scratch("swift-calls", &[("Session.swift", SWIFT)]);
    let d = |w| defs(&dir, &files, Kind::Swift, w);
    let none = Vec::<usize>::new();
    assert_eq!(d("finished"), none, "`case .finished:` is a pattern");
    assert_eq!(d("start"), none, "a call on `self`");
    assert_eq!(d("print"), none);
    assert_eq!(d("Request"), none, "a type this file only uses");
    assert_eq!(
        d("delegate"),
        none,
        "an `if let` rebinds a name declared elsewhere: no rule, so `u` answers"
    );
    assert_eq!(d("body"), none, "a parameter has no rule");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn swift_scope_roots_and_names() {
    let here = Path::new("Session.swift");
    assert!(in_def_scope(
        Kind::Swift,
        here,
        Path::new("Source/Request.swift")
    ));
    assert!(!in_def_scope(Kind::Swift, here, Path::new("main.rs")));
    // An `import` names a module and makes everything in it visible unqualified, so it binds
    // no name of its own.
    assert!(imports(Kind::Swift, SWIFT).is_empty());
    // Outside the project is where SwiftPM checks the dependencies out; nothing else on the
    // machine holds Swift source, so an absent directory leaves the list empty.
    let (dir, _) = scratch(
        "swift-roots",
        &[(".build/checkouts/nio/Sources/a.swift", "")],
    );
    assert_eq!(
        external_roots(Kind::Swift, &dir),
        [dir.join(".build/checkouts")]
    );
    assert!(external_roots(Kind::Swift, Path::new("/")).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
    // A member is named by the type its extension extends.
    assert_eq!(
        qualified(Kind::Swift, SWIFT, 41, "didFinish").as_deref(),
        Some("Session.didFinish")
    );
}

const PHP: &str = r#"<?php

namespace App\Services;

use Illuminate\Support\Str;
use App\Models\User as Account;

define('BILLING_LIMIT', 10);

abstract class Invoice implements Arrayable
{
    public const STATUS_OPEN = 'open';

    protected array $rows = [];

    private ?Logger $logger;

    public function __construct(private readonly Account $account, string $name)
    {
        $this->logger = null;
        $total = 0;
        foreach ($this->rows as $key => $value) {
            $total += $value;
        }
        $this->name = $name;
    }

    final public static function parse(string $text): static
    {
        return new static($text);
    }

    public function &rows(): array
    {
        return $this->rows;
    }

    abstract protected function compute(): int;
}

interface Arrayable
{
    public function toArray(): array;
}

trait Macroable
{
    public function macro(string $name): void
    {
    }
}

enum Status: string
{
    case Open = 'open';
    case Closed;
}

function billing_total(Invoice $invoice): int
{
    $sum = 0;
    return $sum;
}

function billing_report(array $rows, int $total): array
{
    $map = [
        $key => $value,
    ];

    return
        $total == 0 ? $map : $rows;
}
"#;

#[test]
fn php_def_patterns_find_declarations_behind_modifiers() {
    let (dir, files) = scratch("php", &[("Invoice.php", PHP)]);
    let d = |w| defs(&dir, &files, Kind::Php, w);
    assert_eq!(d("Services"), [3], "a namespace, by its last part");
    assert_eq!(d("BILLING_LIMIT"), [8], "a `define()` constant");
    assert_eq!(d("Invoice"), [10], "not the `Invoice $invoice` parameter");
    assert_eq!(d("STATUS_OPEN"), [12], "a class constant");
    // The property and the method that returns it, not the `$this->rows` uses.
    assert_eq!(d("rows"), [14, 33]);
    assert_eq!(d("logger"), [16], "not the `$this->logger = null;` write");
    assert_eq!(d("account"), [18], "a promoted constructor parameter");
    assert_eq!(
        d("total"),
        [21, 23],
        "the assignment and the `+=` that follows"
    );
    assert_eq!(d("parse"), [28], "behind `final public static`");
    assert_eq!(d("compute"), [38], "an abstract method has no body");
    assert_eq!(d("Arrayable"), [41], "not the `implements Arrayable`");
    assert_eq!(d("toArray"), [43]);
    assert_eq!(d("Macroable"), [46], "a trait");
    assert_eq!(d("macro"), [48]);
    assert_eq!(d("Status"), [53], "a backed enum");
    assert_eq!(d("Open"), [55], "an enum case with its value");
    assert_eq!(d("Closed"), [56]);
    assert_eq!(d("billing_total"), [59], "a function at the top level");
    assert_eq!(d("sum"), [61], "not the `return $sum;`");
    assert_eq!(d("map"), [67]);
    // Not the `$total == 0` that opens line 72: `==` compares, it declares nothing.
    assert_eq!(d("total"), [21, 23]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn php_def_patterns_tell_a_declaration_from_a_use() {
    let (dir, files) = scratch("php-uses", &[("Invoice.php", PHP)]);
    let d = |w| defs(&dir, &files, Kind::Php, w);
    let none = Vec::<usize>::new();
    assert_eq!(d("Str"), none, "a `use` imports a name, it declares none");
    assert_eq!(d("Account"), none, "nor does the alias of one");
    assert_eq!(d("Logger"), none, "a type a property is written with");
    assert_eq!(d("text"), none, "a parameter has no rule");
    assert_eq!(
        d("name"),
        none,
        "`$this->name = $name;` writes to a property declared elsewhere"
    );
    assert_eq!(
        d("key"),
        none,
        "a `foreach` target has no rule, and `$key => $value,` is an array pair"
    );
    assert_eq!(d("value"), none);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn php_scope_roots_imports_and_names() {
    let here = Path::new("src/Invoice.php");
    assert!(in_def_scope(Kind::Php, here, Path::new("views/show.phtml")));
    assert!(!in_def_scope(Kind::Php, here, Path::new("main.rs")));
    // A `use` in column zero binds the last part of the path, or its alias; PSR-4 spells the
    // namespace the way the file system does, so the path is the name split on `\`.
    assert_eq!(
        imports(Kind::Php, PHP),
        [
            (
                "Str".to_owned(),
                vec!["Illuminate".into(), "Support".into(), "Str".into()]
            ),
            (
                "Account".to_owned(),
                vec!["App".into(), "Models".into(), "User".into()]
            ),
        ]
    );
    // An indented `use` pulls a trait into a class body and names no file.
    assert!(imports(Kind::Php, "class X {\n    use Macroable;\n}\n").is_empty());
    // Composer installs the dependencies into `vendor/`, which is gitignored and so outside
    // the project walk, the way `node_modules` is.
    let (dir, _) = scratch("php-roots", &[("vendor/laravel/framework/src/a.php", "")]);
    assert_eq!(external_roots(Kind::Php, &dir), [dir.join("vendor")]);
    assert!(external_roots(Kind::Php, Path::new("/")).is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
    // PHP writes a method of a class behind `::`, as its own documentation does.
    assert_eq!(
        qualified(Kind::Php, PHP, 28, "parse").as_deref(),
        Some("Invoice::parse")
    );
}

const LUA: &str = r#"local uv = vim.uv

local M = {}
local cache, hits = {}, 0

function M.setup(opts)
  local defaults = { limit = 10 }
  cache = defaults
  return M.normalise(opts)
end

function M:render(row)
  return row
end

function normalise(opts)
  return opts
end

local function trim(s)
  return s
end

M.format = function(row)
  return trim(row)
end

local handlers = {
  open = function(id)
    return id
  end,
  limit = 10,
}

--[[
function M.ghost(x)
  return x
end
]]

M.setup({ limit = 1 })
return M

local pat = [=[
^\s*\%(\[[A-Za-z]\+\]\)* ]-] x
function M.ghosted(x)
end
]=]

function M.after(x)
  return x
end

local sql = [[
function M.ghost2(x)
end
]]

function M.last() end
"#;

#[test]
fn lua_def_patterns_find_functions_and_locals() {
    let (dir, files) = scratch("lua", &[("init.lua", LUA)]);
    let d = |w| defs(&dir, &files, Kind::Lua, w);
    assert_eq!(d("setup"), [6], "the declaration, not the call on line 41");
    assert_eq!(d("render"), [12], "the `M:name` form");
    assert_eq!(d("normalise"), [16], "not the `M.normalise(opts)` call");
    assert_eq!(d("trim"), [20], "`local function`");
    assert_eq!(d("format"), [24], "`M.name = function`");
    assert_eq!(d("open"), [29], "a function in a table of handlers");
    assert_eq!(d("M"), [3]);
    assert_eq!(d("uv"), [1]);
    // `local a, b = …` declares both, and a later bare `cache = …` is an assignment to the
    // local already declared, not a declaration of its own.
    assert_eq!(d("cache"), [4]);
    assert_eq!(d("hits"), [4]);
    assert_eq!(d("defaults"), [7], "a local inside a body");
    assert_eq!(
        d("limit"),
        Vec::<usize>::new(),
        "a table field holding a value has no rule: the line is also an assignment"
    );
    assert_eq!(d("opts"), Vec::<usize>::new(), "a parameter");
    assert_eq!(d("row"), Vec::<usize>::new());
    assert_eq!(
        d("vim"),
        Vec::<usize>::new(),
        "the right-hand side of a local"
    );
    // A `[=[ … ]=]` long string closes on the `=` it was opened with, so neither the
    // `\[[` of the Vim regex inside it nor the `]-]` closes it, and what follows the
    // string is still read as code.
    assert_eq!(d("pat"), [44]);
    assert_eq!(d("after"), [50]);
    assert_eq!(d("last"), [59]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn lua_long_brackets_hide_what_they_hold() {
    // The `--[[ … ]]` block comment on lines 35-39, the `[=[ … ]=]` string on 44-48 and
    // the `[[ … ]]` one on 54-57: the functions inside them declare nothing, the way a
    // Python docstring's example does not.
    let lit = literal_lines(Kind::Lua, LUA);
    assert_eq!(
        lit.iter()
            .enumerate()
            .filter(|(_, l)| **l)
            .map(|(i, _)| i + 1)
            .collect::<Vec<_>>(),
        [36, 37, 38, 39, 45, 46, 47, 48, 55, 56, 57]
    );
    // A `--` line comment is still one line, whatever quote it holds.
    assert!(
        literal_lines(Kind::Lua, "-- don't\nlocal x = 1\n")[1..]
            .iter()
            .all(|l| !l)
    );
}

#[test]
fn lua_scope_roots_and_names() {
    let here = Path::new("lua/config/init.lua");
    assert!(in_def_scope(
        Kind::Lua,
        here,
        Path::new("lua/plugins/ui.lua")
    ));
    assert!(!in_def_scope(Kind::Lua, here, Path::new("main.c")));
    // `require "x"` binds a name, but there is no root to resolve it against and Lua's own
    // `package.path` is the embedding interpreter's, so nothing is bound and nothing is
    // searched outside the project.
    assert!(imports(Kind::Lua, LUA).is_empty());
    assert!(external_roots(Kind::Lua, Path::new("/")).is_empty());
    assert!(member_patterns(Kind::Lua, "setup").is_none());
    // A function nested in another is named under it, as in every kind, and a `--`
    // comment in between is a comment, not a declaration that names nothing.
    assert_eq!(
        qualified(
            Kind::Lua,
            "function M.setup()\n-- a note\n  local function inner() end\nend\n",
            3,
            "inner"
        )
        .as_deref(),
        Some("setup.inner")
    );
}

#[test]
fn lua_symbol_names() {
    let lua = |line| one(Kind::Lua, line);
    for (line, name) in [
        ("function setup(opts)", Some("setup")),
        ("function M.setup(opts)", Some("setup")),
        ("function M:render(row)", Some("render")),
        ("function vim.lsp.util.clamp(x)", Some("clamp")),
        ("local function trim(s)", Some("trim")),
        ("  local function inner()", Some("inner")),
        ("M.format = function(row)", Some("format")),
        ("local format = function(row)", Some("format")),
        ("  open = function(id)", Some("open")),
        // Not a declaration: a call, a field holding a value, a local, a return.
        ("M.setup({ limit = 1 })", None),
        ("  limit = 10,", None),
        ("local M = {}", None),
        ("local cache, hits = {}, 0", None),
        ("  return M.normalise(opts)", None),
        ("  end,", None),
        ("-- function ghost(x)", None),
    ] {
        assert_eq!(lua(line).as_deref(), name, "{line}");
    }
}

const EX: &str = r#"defmodule MyApp.Ledger do
  @moduledoc """
  Examples:

      def ghost(x), do: x
  """

  @timeout 5_000
  @derive {Jason.Encoder, only: [:id]}

  defstruct [:id, :total, currency: "EUR"]

  @type t :: %__MODULE__{}

  @spec parse(String.t()) :: t
  def parse(nil), do: nil

  def parse(raw) when is_binary(raw) do
    %__MODULE__{id: raw}
  end

  defp normalise(raw) do
    String.trim(raw)
  end

  defmacro with_total(do: block) do
    block
  end

  defguard is_positive(n) when n > 0

  defdelegate encode(value), to: Jason

  def timeout, do: @timeout
end

defprotocol Renderable do
  def render(value)
end

defimpl Renderable, for: MyApp.Ledger do
  def render(ledger), do: ledger.id
end

defmodule MyApp.LedgerTest do
  @moduletag :slow
  @tag :external

  defmacrop guard!(x), do: x
  defguardp is_even(n) when rem(n, 2) == 0

  def empty?(rows), do: rows == []
  def put!(row), do: row
end
"#;

#[test]
fn elixir_def_patterns_find_every_def_form() {
    let (dir, files) = scratch("ex", &[("ledger.ex", EX)]);
    let d = |w| defs(&dir, &files, Kind::Elixir, w);
    assert_eq!(
        d("Ledger"),
        [1],
        "the last part of `defmodule MyApp.Ledger`"
    );
    assert_eq!(d("Renderable"), [37], "not the `defimpl` that uses it");
    // Two clauses of one function are two declarations, so both are offered; the `@spec`
    // above them is a promise about `parse`, not its definition.
    assert_eq!(d("parse"), [16, 18]);
    assert_eq!(d("normalise"), [22], "`defp`");
    assert_eq!(d("with_total"), [26], "`defmacro`");
    assert_eq!(d("is_positive"), [30], "`defguard`");
    assert_eq!(d("encode"), [32], "`defdelegate`");
    assert_eq!(d("render"), [38, 42], "the protocol and its implementation");
    // The attribute and the function of the same name are both declarations, of different
    // things, so `d` offers both rather than guessing.
    assert_eq!(d("timeout"), [8, 34]);
    assert_eq!(d("id"), [11], "a struct field, atom list form");
    assert_eq!(d("currency"), [11], "the keyword form of the same line");
    // The attributes the language owns, and the names they talk about.
    assert_eq!(d("t"), Vec::<usize>::new(), "`@type t ::` declares no `t`");
    assert_eq!(d("spec"), Vec::<usize>::new());
    assert_eq!(d("type"), Vec::<usize>::new());
    assert_eq!(d("moduledoc"), Vec::<usize>::new());
    assert_eq!(d("derive"), Vec::<usize>::new());
    assert_eq!(d("MyApp"), Vec::<usize>::new(), "a namespace, not a module");
    assert_eq!(d("raw"), Vec::<usize>::new(), "a parameter");
    assert_eq!(d("block"), Vec::<usize>::new());
    assert_eq!(d("Jason"), Vec::<usize>::new());
    assert_eq!(d("guard"), [49], "`defmacrop`, past the trailing `!`");
    assert_eq!(d("is_even"), [50], "`defguardp`");
    // A name Elixir spells with a trailing `?` or `!` is found from the bare word, as
    // Ruby's is: the cursor on `empty` in `empty?(rows)` reaches `def empty?`.
    assert_eq!(d("empty"), [52]);
    assert_eq!(d("put"), [53]);
    // ExUnit's and Mix's attributes are directives too, so `d` on one has nothing to find
    // rather than a picker of every place the directive is written.
    assert_eq!(d("tag"), Vec::<usize>::new());
    assert_eq!(d("moduletag"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn elixir_heredocs_hide_what_they_hold() {
    // `@moduledoc """ … """` on lines 2-6: the `def ghost(x)` of its example declares
    // nothing, as a Python docstring's does not.
    let lit = literal_lines(Kind::Elixir, EX);
    assert_eq!(
        lit.iter()
            .enumerate()
            .filter(|(_, l)| **l)
            .map(|(i, _)| i + 1)
            .collect::<Vec<_>>(),
        [3, 4, 5, 6]
    );
}

#[test]
fn elixir_scope_roots_and_names() {
    let here = Path::new("lib/my_app/ledger.ex");
    assert!(in_def_scope(
        Kind::Elixir,
        here,
        Path::new("test/ledger_test.exs")
    ));
    assert!(!in_def_scope(Kind::Elixir, here, Path::new("mix.lock")));
    // `alias` and `import` bind names, but `mix` puts the dependencies in `deps/` inside the
    // project, so they are project files already and there is no root to leave for.
    assert!(imports(Kind::Elixir, EX).is_empty());
    assert!(external_roots(Kind::Elixir, Path::new("/")).is_empty());
    assert!(member_patterns(Kind::Elixir, "parse").is_none());
    // A function is named under the module it is written in, as in every kind.
    assert_eq!(
        qualified(Kind::Elixir, EX, 22, "normalise").as_deref(),
        Some("Ledger.normalise")
    );
    assert_eq!(qualified(Kind::Elixir, EX, 1, "Ledger"), None);
}

#[test]
fn elixir_symbol_names() {
    let ex = |line| one(Kind::Elixir, line);
    for (line, name) in [
        ("defmodule MyApp.Ledger do", Some("Ledger")),
        ("defmodule Ledger do", Some("Ledger")),
        ("defprotocol Renderable do", Some("Renderable")),
        ("  def parse(nil), do: nil", Some("parse")),
        ("  def timeout, do: @timeout", Some("timeout")),
        ("  defp normalise(raw) do", Some("normalise")),
        ("  def empty?(rows), do: rows == []", Some("empty?")),
        ("  def put!(row), do: row", Some("put!")),
        ("  defmacro with_total(do: block) do", Some("with_total")),
        ("  defmacrop guard!(x), do: x", Some("guard!")),
        ("  defguard is_positive(n) when n > 0", Some("is_positive")),
        (
            "  defguardp is_even(n) when rem(n, 2) == 0",
            Some("is_even"),
        ),
        ("  defdelegate encode(value), to: Jason", Some("encode")),
        // A `defimpl` names the module `Protocol.Type`, and neither half is its own name;
        // `defstruct` declares every field on one line; an attribute belongs to the language.
        ("defimpl Renderable, for: MyApp.Ledger do", None),
        ("  defstruct [:id, :total]", None),
        ("  @spec parse(String.t()) :: t", None),
        ("  @type t :: %__MODULE__{}", None),
        ("  @moduledoc \"\"\"", None),
        ("  @timeout 5_000", None),
        // The shared pattern called this a declaration of `x`.
        ("    Enum.map(rows, fn x -> x.id end)", None),
        ("    String.trim(raw)", None),
        ("  end", None),
    ] {
        assert_eq!(ex(line).as_deref(), name, "{line}");
    }
}

const ZIG: &str = r#"const std = @import("std");
const Allocator = std.mem.Allocator;

pub const Error = error{OutOfRange};

pub const Ledger = struct {
    total: u32,
    rows: []const Row,

    const empty: Ledger = .{ .total = 0, .rows = &.{} };

    pub fn init(allocator: Allocator) Ledger {
        var self = Ledger{ .total = 0, .rows = &.{} };
        return self;
    }

    pub inline fn isEmpty(self: Ledger) bool {
        return self.rows.len == 0;
    }

    fn compute(self: Ledger) u32 {
        return self.total;
    }
};

pub const Row = struct { id: u32 };

const Status = enum { open, closed };

const Value = union(enum) { n: u32, s: []const u8 };

pub var counter: u32 = 0;
threadlocal var scratch: [16]u8 = undefined;

export fn ledger_total(l: *Ledger) u32 {
    return l.total;
}

pub extern "c" fn strlen(s: [*:0]const u8) usize;

noinline fn slow(x: u32) u32 {
    return x;
}

test "a ledger starts empty" {
    const l = Ledger.init(std.testing.allocator);
    try std.testing.expect(l.isEmpty());
}

const first, const second = .{ 1, 2 };

extern fn puts(s: [*:0]const u8) c_int;

export inline fn fast(x: u32) u32 {
    comptime var seen: u32 = 0;
    seen += x;
    return seen;
}

const help =
    \\```zig
    \\const x = 1;
    \\```
;

pub fn after() void {}
"#;

#[test]
fn zig_def_patterns_find_functions_types_and_constants() {
    let (dir, files) = scratch("zig", &[("ledger.zig", ZIG)]);
    let d = |w| defs(&dir, &files, Kind::Zig, w);
    assert_eq!(d("std"), [1]);
    assert_eq!(d("Allocator"), [2]);
    assert_eq!(d("Error"), [4]);
    assert_eq!(
        d("Ledger"),
        [6],
        "not the literal on line 13 or the call on line 46"
    );
    assert_eq!(d("empty"), [10], "a constant in a struct body");
    assert_eq!(d("init"), [12], "not the `Ledger.init(…)` call on line 46");
    assert_eq!(d("isEmpty"), [17], "`pub inline fn`");
    assert_eq!(d("compute"), [21]);
    assert_eq!(
        d("Row"),
        [26],
        "not the `rows: []const Row` field that uses it"
    );
    assert_eq!(d("Status"), [28], "`const X = enum`");
    assert_eq!(d("Value"), [30], "`const X = union(enum)`");
    assert_eq!(d("counter"), [32], "`pub var`");
    assert_eq!(d("scratch"), [33], "`threadlocal var`");
    assert_eq!(d("ledger_total"), [35], "`export fn`");
    assert_eq!(d("strlen"), [39], r#"`pub extern "c" fn`"#);
    assert_eq!(d("slow"), [41], "`noinline fn`");
    assert_eq!(
        d("self"),
        [13],
        "a local; the parameters of lines 17 and 21 are not"
    );
    assert_eq!(d("l"), [46]);
    assert_eq!(
        d("total"),
        Vec::<usize>::new(),
        "a struct field has no rule"
    );
    assert_eq!(d("id"), Vec::<usize>::new());
    assert_eq!(
        d("ledger"),
        Vec::<usize>::new(),
        "a word inside a test description declares nothing"
    );
    assert_eq!(d("open"), Vec::<usize>::new(), "an enum field");
    // A destructuring declares both names, but only the first one starts the line, and every
    // rule here is anchored there.
    assert_eq!(d("first"), [50]);
    assert_eq!(d("second"), Vec::<usize>::new());
    assert_eq!(d("puts"), [52], "`extern fn`, with no calling convention");
    assert_eq!(d("fast"), [54], "`export inline fn`");
    assert_eq!(d("seen"), [55], "`comptime var`");
    // Zig has no literal that runs over lines: a `\\` string ends with its line, so the
    // markdown fences on 61-63 open nothing and the declaration below them is still found.
    assert_eq!(d("after"), [66]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn zig_has_no_literal_that_runs_over_lines() {
    // A `\\` string holding markdown is idiomatic in Zig, and its ``` fences are not a
    // TypeScript template: reading Zig with the C family's rules would hide every line
    // after the first fence, and `d` would say `no definition` over code it can see.
    assert!(
        literal_lines(Kind::Zig, ZIG).iter().all(|l| !l),
        "a Zig line was taken for the inside of a literal"
    );
}

#[test]
fn zig_scope_roots_and_names() {
    let here = Path::new("src/main.zig");
    assert!(in_def_scope(Kind::Zig, here, Path::new("src/ledger.zig")));
    assert!(!in_def_scope(Kind::Zig, here, Path::new("build.zig.zon")));
    assert!(imports(Kind::Zig, ZIG).is_empty());
    assert!(member_patterns(Kind::Zig, "init").is_none());
    // The standard library `zig env` reports, on a machine that has a `zig`; nothing at all
    // on one that does not, as for every kind whose toolchain is not installed.
    assert!(
        external_roots(Kind::Zig, Path::new("/"))
            .iter()
            .all(|r| r.is_dir() && r.ends_with("std")),
        "a Zig root that is not an existing `std` directory"
    );
    // A method is named under the type it is declared in, as in every kind.
    assert_eq!(
        qualified(Kind::Zig, ZIG, 12, "init").as_deref(),
        Some("Ledger.init")
    );
    assert_eq!(qualified(Kind::Zig, ZIG, 6, "Ledger"), None);
}

#[test]
fn zig_std_comes_from_zig_env() {
    // What `zig env` prints: JSON on some versions, ZON on others.
    let json = "{\n \"zig_exe\": \"/opt/homebrew/bin/zig\",\n \"lib_dir\": \"/opt/lib/zig\",\n \"std_dir\": \"/opt/lib/zig/std\"\n}\n";
    assert_eq!(zig_roots(json), [PathBuf::from("/opt/lib/zig/std")]);
    let zon = ".{ .zig_exe = \"/usr/bin/zig\", .lib_dir = \"/usr/lib/zig\", .std_dir = \"/usr/lib/zig/std\" }\n";
    assert_eq!(zig_roots(zon), [PathBuf::from("/usr/lib/zig/std")]);
    // A version that reports only the library directory the standard library sits in.
    assert_eq!(
        zig_roots("{\"lib_dir\": \"/usr/lib/zig\"}"),
        [PathBuf::from("/usr/lib/zig/std")]
    );
    // No `zig` on this machine: nothing to search outside the project.
    assert!(zig_roots("").is_empty());
}

#[test]
fn zig_symbol_names() {
    let zig = |line| one(Kind::Zig, line);
    for (line, name) in [
        // The shared pattern reads these; the rows of this kind must not list them again.
        ("pub const Ledger = struct {", Some("Ledger")),
        ("const Status = enum { open, closed };", Some("Status")),
        ("const Value = union(enum) { n: u32 };", Some("Value")),
        (
            "    pub fn init(allocator: Allocator) Ledger {",
            Some("init"),
        ),
        ("    fn compute(self: Ledger) u32 {", Some("compute")),
        (
            "export fn ledger_total(l: *Ledger) u32 {",
            Some("ledger_total"),
        ),
        (
            "pub extern \"c\" fn strlen(s: [*:0]const u8) usize;",
            Some("strlen"),
        ),
        // These it has no word for.
        (
            "    pub inline fn isEmpty(self: Ledger) bool {",
            Some("isEmpty"),
        ),
        ("noinline fn slow(x: u32) u32 {", Some("slow")),
        (
            "export inline fn ledger_total(l: *Ledger) u32 {",
            Some("ledger_total"),
        ),
        (
            "pub extern \"c\" inline fn strlen(s: [*:0]const u8) usize;",
            Some("strlen"),
        ),
        (
            "test \"a ledger starts empty\" {",
            Some("a ledger starts empty"),
        ),
        // A global, a local and a field stay off the list, as in every other kind.
        ("pub var counter: u32 = 0;", None),
        ("threadlocal var scratch: [16]u8 = undefined;", None),
        ("        var self = Ledger{ .total = 0 };", None),
        ("    const empty: Ledger = .{ .total = 0 };", None),
        ("    total: u32,", None),
        ("    return self.total;", None),
        ("    try std.testing.expect(l.isEmpty());", None),
    ] {
        assert_eq!(zig(line).as_deref(), name, "{line}");
    }
}

const SH: &str = "#!/usr/bin/env bash\nset -eu\n\nexport ROOT=/srv\nlocal -i tries=3\ndeclare -r -x LIMIT=10\nreadonly NAME=app\nPATH+=:/opt/bin\nalias ll='ls -l'\n\nbuild() {\n  echo \"$ROOT\"\n}\n\nfunction deploy {\n  build\n}\n\nfunction check() {\n  [ \"$NAME\" = app ]\n}\n\nbuild \"$ROOT\"\n";

#[test]
fn shell_def_patterns_find_functions_assignments_and_aliases() {
    let (dir, files) = scratch("sh", &[("run.sh", SH)]);
    let d = |w| defs(&dir, &files, Kind::Shell, w);
    // The definition, not the `build` call on line 16 or line 23.
    assert_eq!(d("build"), [11]);
    assert_eq!(d("deploy"), [15]);
    assert_eq!(d("check"), [19], "`function name()` counts once");
    assert_eq!(d("ROOT"), [4], "not the `\"$ROOT\"` uses");
    assert_eq!(d("tries"), [5]);
    assert_eq!(d("LIMIT"), [6], "behind `declare` and its flags");
    // The `readonly` assignment, not the `[ \"$NAME\" = app ]` test.
    assert_eq!(d("NAME"), [7]);
    assert_eq!(d("PATH"), [8], "`+=` appends to a variable");
    assert_eq!(d("ll"), [9]);
    assert_eq!(d("echo"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

const SQL: &str = r#"CREATE TABLE public.orders (
  id serial PRIMARY KEY,
  customer_id int REFERENCES customers(id)
);

CREATE OR REPLACE FUNCTION total(o int) RETURNS int AS $$ SELECT 0 $$ LANGUAGE sql;

CREATE UNIQUE INDEX orders_id_idx ON public.orders (id);

create materialized view daily_totals as select 1;

CREATE TYPE mood AS ENUM ('ok', 'bad');

CREATE TABLE IF NOT EXISTS billing.invoices (id int);

CREATE TABLE "user" (id int);

WITH recent AS (
  SELECT * FROM public.orders
), older AS (
  SELECT * FROM archive
)
SELECT * FROM recent JOIN older ON true;

SELECT * FROM customers;
JOIN customers ON true
INSERT INTO customers VALUES (1);
ALTER TABLE customers ADD COLUMN x int;
DROP TABLE customers;
"#;

#[test]
fn sql_def_patterns_find_create_statements_and_ctes() {
    let (dir, files) = scratch("sql", &[("schema.sql", SQL)]);
    let d = |w| defs(&dir, &files, Kind::Sql, w);
    // The bare name finds the schema-qualified `CREATE TABLE`.
    assert_eq!(d("orders"), [1]);
    assert_eq!(d("total"), [6]);
    assert_eq!(d("orders_id_idx"), [8]);
    // Lower-case keywords read the same.
    assert_eq!(d("daily_totals"), [10]);
    assert_eq!(d("mood"), [12]);
    assert_eq!(d("invoices"), [14]);
    assert_eq!(d("user"), [16], "a quoted name");
    // The `WITH` and the `,` continuation both open a CTE.
    assert_eq!(d("recent"), [18]);
    assert_eq!(d("older"), [20]);
    // `customers` is only ever used, never created: no definition.
    assert_eq!(d("customers"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

const MAKE: &str = ".PHONY: build test\nCC ?= gcc\nexport CFLAGS := -O2\nbuild test-all: deps\n\t$(CC) -o app\ndeps::\n\t@echo deps\n%.o: %.c\n";

#[test]
fn make_def_patterns_find_targets_and_variables() {
    let (dir, files) = scratch("make", &[("Makefile", MAKE)]);
    let d = |w| defs(&dir, &files, Kind::Make, w);
    assert_eq!(d("build"), [4]);
    assert_eq!(d("test-all"), [4], "one of several targets");
    // The `deps::` rule, not the prerequisite on line 4.
    assert_eq!(d("deps"), [6]);
    assert_eq!(d("CC"), [2]);
    assert_eq!(d("CFLAGS"), [3]);
    assert_eq!(d("gcc"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

const TF: &str = r#"variable "region" {
  default = "eu"
}
locals {
  name = "logs-${var.region}"
  tags = {
    name = "x"
  }
}
resource "aws_s3_bucket" "logs" {
  bucket = local.name
}
data "aws_ami" "logs" {
  most_recent = true
}
module "vpc" {
  source = "./vpc"
}
output "bucket" {
  value = aws_s3_bucket.logs.id
}
"#;

#[test]
fn terraform_def_patterns_resolve_the_address() {
    let (dir, files) = scratch("tf", &[("main.tf", TF)]);
    let d = |w| defs(&dir, &files, Kind::Terraform, w);
    assert_eq!(d("var.region"), [1]);
    assert_eq!(d("aws_s3_bucket.logs.id"), [10]);
    assert_eq!(d("data.aws_ami.logs"), [13]);
    assert_eq!(d("module.vpc.cidr"), [16]);
    // A bare name is any block with that label, as from a `.tfvars` file.
    assert_eq!(d("logs"), [10, 13]);
    assert_eq!(d("region"), [1]);
    // `local.name` matches every `name =`; only the one directly in `locals` survives.
    assert_eq!(d("local.name"), [5, 7]);
    assert_eq!(def_block(Kind::Terraform, "local.name"), Some("locals"));
    assert_eq!(def_block(Kind::Terraform, "var.name"), None);
    assert!(directly_inside(TF, 5, "locals"));
    assert!(!directly_inside(TF, 7, "locals"), "nested in `tags`");
    assert!(!directly_inside(TF, 11, "locals"));
    assert!(!directly_inside(TF, 1, "locals"));
    assert!(!directly_inside(TF, 0, "locals"));
    assert!(def_patterns(Kind::Terraform, "each.key").is_empty());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn docker_and_yaml_def_patterns() {
    let docker = "FROM rust:1.80 AS build\nRUN cargo build\nFROM --platform=$BUILDPLATFORM debian AS Runtime\nCOPY --from=build /x /y\n";
    let yaml = "x-common: &common\n  restart: always\nservices:\n  web:\n    <<: *common\n    depends_on: [db-main]\n  db-main:  # the database\n    image: postgres\n.base:\n  script: make\n";
    let (dir, files) = scratch("dy", &[("Dockerfile", docker), ("compose.yml", yaml)]);
    let (docker, yaml) = (files[..1].to_vec(), files[1..].to_vec());
    assert_eq!(defs(&dir, &docker, Kind::Docker, "build"), [1]);
    assert_eq!(defs(&dir, &docker, Kind::Docker, "runtime"), [3]);
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "common"), [1]);
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "db-main"), [7]);
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "web"), [4]);
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "base"), [9]);
    // A key with a value on its line is data, not a definition.
    assert_eq!(defs(&dir, &yaml, Kind::Yaml, "image"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn kinds_come_from_the_file_name() {
    for (name, kind) in [
        ("a.py", Some(Kind::Python)),
        ("a.go", Some(Kind::Go)),
        ("lib.rs", Some(Kind::Rust)),
        ("app.tsx", Some(Kind::TsJs)),
        ("util.cjs", Some(Kind::TsJs)),
        ("Invoice.java", Some(Kind::Jvm)),
        ("invoice.rb", Some(Kind::Ruby)),
        ("Rakefile", Some(Kind::Ruby)),
        ("Gemfile", Some(Kind::Ruby)),
        ("config.ru", Some(Kind::Ruby)),
        ("merl.gemspec", Some(Kind::Ruby)),
        ("tasks.rake", Some(Kind::Ruby)),
        ("invoice.rbi", Some(Kind::Ruby)),
        ("invoice.c", Some(Kind::C)),
        ("invoice.h", Some(Kind::C)),
        ("ledger.cc", Some(Kind::C)),
        ("ledger.cpp", Some(Kind::C)),
        ("ledger.cxx", Some(Kind::C)),
        ("ledger.hpp", Some(Kind::C)),
        ("ledger.hh", Some(Kind::C)),
        ("ledger.hxx", Some(Kind::C)),
        ("Invoice.cs", Some(Kind::CSharp)),
        ("build.csx", Some(Kind::CSharp)),
        ("Session.swift", Some(Kind::Swift)),
        ("Invoice.php", Some(Kind::Php)),
        ("show.phtml", Some(Kind::Php)),
        ("init.lua", Some(Kind::Lua)),
        ("ledger.ex", Some(Kind::Elixir)),
        ("mix.exs", Some(Kind::Elixir)),
        ("ledger.zig", Some(Kind::Zig)),
        // Zig's data format: painted as Zig, but it declares nothing.
        ("build.zig.zon", None),
        ("app.kt", Some(Kind::Jvm)),
        ("build.gradle.kts", Some(Kind::Jvm)),
        ("run.sh", Some(Kind::Shell)),
        ("run.bash", Some(Kind::Shell)),
        ("run.zsh", Some(Kind::Shell)),
        ("run.ksh", Some(Kind::Shell)),
        (".bashrc", Some(Kind::Shell)),
        (".bash_profile", Some(Kind::Shell)),
        (".bash_aliases", Some(Kind::Shell)),
        (".zshrc", Some(Kind::Shell)),
        (".zshenv", Some(Kind::Shell)),
        (".zprofile", Some(Kind::Shell)),
        (".profile", Some(Kind::Shell)),
        // A shebang-only script: `kind_of` goes by the name, so it has no kind.
        ("install", None),
        ("schema.sql", Some(Kind::Sql)),
        ("dump.psql", Some(Kind::Sql)),
        ("init.pgsql", Some(Kind::Sql)),
        ("seed.mysql", Some(Kind::Sql)),
        ("001.ddl", Some(Kind::Sql)),
        ("002.dml", Some(Kind::Sql)),
        ("Makefile", Some(Kind::Make)),
        ("GNUmakefile", Some(Kind::Make)),
        ("rules.mk", Some(Kind::Make)),
        ("main.tf", Some(Kind::Terraform)),
        ("prod.tfvars", Some(Kind::Terraform)),
        ("Dockerfile", Some(Kind::Docker)),
        ("Dockerfile.prod", Some(Kind::Docker)),
        ("api.Dockerfile", Some(Kind::Docker)),
        ("Containerfile", Some(Kind::Docker)),
        ("Dockerfile.dockerignore", None),
        ("compose.yaml", Some(Kind::Yaml)),
        (".gitlab-ci.yml", Some(Kind::Yaml)),
        ("README", None),
        ("notes.txt", None),
    ] {
        assert_eq!(kind_of(Path::new(name)), kind, "{name}");
    }
}

#[test]
fn qualifier_is_the_chain_in_front_of_the_word() {
    let q = |l: &str, i| qualifier(l, i);
    assert_eq!(q("    return json.load(fp)", 16), ["json"]);
    assert_eq!(q("x = os.path.join(a)", 12), ["os", "path"]);
    assert_eq!(q("let s = fs::read_to_string(p)", 12), ["fs"]);
    assert!(q("load(fp)", 0).is_empty());
    assert!(q("x = load(fp)", 4).is_empty());
    assert_eq!(q("this.#root.insert(p)", 11), ["this", "#root"]);
    // A chain that hangs off a call, an index or `?.` has no name to start from.
    assert!(q("x = make_uow().users.delete_user()", 21).is_empty());
    assert!(q("a[0].users.find()", 11).is_empty());
    assert!(q("a?.users.find()", 9).is_empty());
    assert!(q("  .users.find()", 9).is_empty());
    // Python's `super()` is one name, as TypeScript's `super` is; `my_super()` is a call.
    assert_eq!(q("        super().store(item)", 16), ["super"]);
    assert_eq!(q("    print(super().label)", 18), ["super"]);
    assert!(q("    my_super().store(item)", 15).is_empty());
    assert!(q("    a.super().store(item)", 14).is_empty());
    // A spread and a range are no member access.
    assert_eq!(q("f(...this.repo.find())", 15), ["this", "repo"]);
    assert_eq!(q("for i in 0..v.len() {", 14), ["v"]);
    // A character outside ASCII in front of the word is no name char, and the chain is read
    // without slicing inside it (#150).
    assert!(q("    данные.load(x)", 17).is_empty());
    assert!(q("  café.load(x)", 8).is_empty());
}

#[test]
fn a_loop_hands_out_elements_only_where_the_type_says_so() {
    let cases = [
        (Kind::Python, "list[Repo]", Some("Repo")),
        (
            Kind::Python,
            "typing.Sequence[models.Repo]",
            Some("models.Repo"),
        ),
        (Kind::Python, "\"tuple[Repo, ...]\"", Some("Repo")),
        (Kind::Python, "set[Repo | None]", Some("Repo | None")),
        // Keys, a fixed tuple, a type of the project's own, no arguments at all.
        (Kind::Python, "dict[str, Repo]", None),
        (Kind::Python, "tuple[Repo, Audit]", None),
        (Kind::Python, "Page[Repo]", None),
        (Kind::Python, "list", None),
        (Kind::TsJs, "Repo[]", Some("Repo")),
        (Kind::TsJs, "readonly Repo[]", Some("Repo")),
        (Kind::TsJs, "Array<Box<Repo>>", Some("Box<Repo>")),
        (Kind::TsJs, "Set<Repo>;", Some("Repo")),
        (Kind::TsJs, "Map<string, Repo>", None),
        (Kind::TsJs, "Promise<Repo[]>", None),
        (Kind::TsJs, "Repo", None),
        (Kind::Go, "[]*Repo", Some("*Repo")),
        (Kind::Go, "[4]Repo", Some("Repo")),
        (Kind::Go, "map[Key]*models.Repo", Some("*models.Repo")),
        (Kind::Go, "map[[2]int][]Repo", Some("[]Repo")),
        (Kind::Go, "chan *Repo", None),
        (Kind::Go, "*[]Repo", None),
        (Kind::Go, "Repos", None),
    ];
    for (kind, written, want) in cases {
        let got = element_type(kind, written);
        assert_eq!(got.as_deref(), want, "{written}");
    }
    // The loops that hand out something else are unknown.
    let element = |name: &str| Value::Element(name.into());
    let py = "async def f(repos):\n    for r in repos:\n        r\n    async for r in repos:\n        r\n    for i, r in pairs:\n        r\n    for r in load():\n        r\n";
    let at = |kind, text, line, name| bound_at(kind, text, line, name);
    assert_eq!(
        at(Kind::Python, py, 3, "r"),
        [
            (2, element("repos")),
            (4, Value::Unknown),
            (6, Value::Unknown),
            (8, Value::Unknown)
        ]
    );
    let ts = "for (const r of repos) {\n  r;\n}\nfor (const k in repos) {\n  k;\n}\nfor (const [k, r] of pairs) {\n  r;\n}\nfor (const r of load()) {\n  r;\n}\n";
    assert_eq!(at(Kind::TsJs, ts, 2, "r"), [(1, element("repos"))]);
    assert_eq!(at(Kind::TsJs, ts, 5, "k"), [(4, Value::Unknown)]);
    assert_eq!(at(Kind::TsJs, ts, 8, "r"), [(7, Value::Unknown)]);
    assert_eq!(at(Kind::TsJs, ts, 11, "r"), [(10, Value::Unknown)]);
    let go = "func f() {\n\tfor _, r := range repos {\n\t\tr.Go()\n\t}\n\tfor i, r := range repos {\n\t\ti.Go()\n\t}\n\tfor r := range repos {\n\t\tr.Go()\n\t}\n\tfor _, r := range s.repos {\n\t\tr.Go()\n\t}\n}\n";
    assert_eq!(at(Kind::Go, go, 3, "r"), [(2, element("repos"))]);
    assert_eq!(at(Kind::Go, go, 6, "i"), [(5, Value::Unknown)]);
    assert_eq!(at(Kind::Go, go, 9, "r"), [(8, Value::Unknown)]);
    assert_eq!(at(Kind::Go, go, 12, "r"), [(11, Value::Unknown)]);
}

/// The bindings of `name` on `line` of `text`, as (line, value) pairs.
fn bound_at(kind: Kind, text: &str, line: usize, name: &str) -> Vec<(usize, Value)> {
    bindings(kind, text, line, name)
        .into_iter()
        .map(|b| (b.line, b.value))
        .collect()
}

fn ty(t: &str) -> Value {
    Value::Type(t.into())
}

#[test]
fn python_bindings_cover_parameters_locals_and_the_class_of_self() {
    let text = r#"from repos import UserRepository

repo = UserRepository()


class Service:
    audit: AuditLog

    def __init__(
        self,
        repo: UserRepository,
        cache: "Cache | None" = None,
    ) -> None:
        self.repo = repo
        local: Optional[Repo] = make()

    @staticmethod
    def build(first, second: int):
        return first.x(second)

    def run(self, items):
        for item in items:
            item.go()
        self.repo.delete_user(1)
        [r.go() for r in items]
        done = lambda repo: repo.close()
        a, b = items
        with open() as fh:
            fh.read()
        log(a, level=1)

import os.path, store.sessions as sessions
"#;
    let at = |line, name| bound_at(Kind::Python, text, line, name);
    // A parameter over a multi-line signature hides the module's `repo` above.
    assert_eq!(at(14, "repo"), [(9, ty("UserRepository"))]);
    assert_eq!(at(1, "repo"), [(3, Value::Call("UserRepository".into()))]);
    assert_eq!(at(14, "cache"), [(9, ty("\"Cache | None\""))]);
    assert_eq!(at(14, "local"), [(15, ty("Optional[Repo]"))]);
    assert_eq!(at(24, "self"), [(21, Value::Class(6))]);
    // A static method's first parameter is no `self`, and an unannotated one is unknown.
    assert_eq!(at(19, "first"), [(18, Value::Unknown)]);
    assert_eq!(at(19, "second"), [(18, ty("int"))]);
    assert_eq!(at(23, "item"), [(22, Value::Element("items".into()))]);
    // A comprehension or a lambda binds on its own line only.
    assert_eq!(at(25, "r"), [(25, Value::Unknown)]);
    assert_eq!(at(24, "r"), []);
    // Unknown where it is written, it hides the module's `repo` and proves nothing.
    assert_eq!(at(26, "repo"), [(26, Value::Unknown)]);
    // A tuple and `as` are unknown; a keyword argument is no binding.
    assert_eq!(at(29, "a"), [(27, Value::Unknown)]);
    assert_eq!(at(29, "fh"), [(28, Value::Unknown)]);
    assert_eq!(at(30, "level"), []);
    // An import binds the names it imports, not the modules on their path.
    assert_eq!(at(14, "UserRepository"), [(1, Value::Unknown)]);
    assert_eq!(at(14, "repos"), []);
    assert_eq!(at(14, "os"), [(32, Value::Unknown)]);
    assert_eq!(at(14, "path"), []);
    assert_eq!(at(14, "sessions"), [(32, Value::Unknown)]);
}

/// Comments and strings hold brackets, quotes and commas that are no code: fastapi writes a
/// `# type: ignore` on a signature line and `Doc("""…""")` texts through its signatures.
#[test]
fn bindings_read_past_comments_strings_and_long_signatures() {
    let filler: String = (0..70).map(|i| format!("    p{i}: int,\n")).collect();
    let py = format!(
        "class Basic(Base):
    async def __call__(  # type: ignore
        self, request: Request  # the request (
    ) -> None:
        repo: Repo  # a comment, with a bracket (
        repo.find(self)


def delete(
    path: Annotated[str, Doc(\"\"\"
        The path (see the docs, it's here.
    \"\"\")] = \"a,(\",
{filler}    repo: Repo = None,
) -> None:
    repo.find()
"
    );
    let at = |line, name| bound_at(Kind::Python, &py, line, name);
    assert_eq!(at(6, "self"), [(2, Value::Class(1))]);
    assert_eq!(at(6, "repo"), [(5, ty("Repo"))]);
    assert_eq!(at(85, "repo"), [(9, ty("Repo"))]);
    assert_eq!(
        at(85, "path"),
        [(
            9,
            ty(
                "Annotated[str, Doc(\"\"\"\n        The path (see the docs, it's here.\n    \"\"\")]"
            )
        )]
    );
    let ts = "export function run(sep = \"a,(\", repo: Repo) {\n  repo.find(sep); // a call (\n}\n";
    assert_eq!(bound_at(Kind::TsJs, ts, 2, "repo"), [(1, ty("Repo"))]);
    assert_eq!(bound_at(Kind::TsJs, ts, 2, "sep"), [(1, Value::Unknown)]);
    let go = "func (s *Service) Do(\n\tctx context.Context, // the context (\n\trepo *Repo,\n) {\n\trepo.Find(ctx)\n}\n";
    assert_eq!(bound_at(Kind::Go, go, 5, "repo"), [(1, ty("*Repo"))]);
    assert_eq!(bound_at(Kind::Go, go, 5, "s"), [(1, ty("*Service"))]);
}

#[test]
fn python_bindings_see_the_enclosing_functions_but_not_the_nested_ones() {
    let text = "def cleanup(user_id: int) -> None:\n    repo = AuditLog()\n\n    async def purge() -> None:\n        repo = UserRepository()\n        await repo.delete_user(user_id)\n\n    repo.delete_user(user_id)\n    repo = make_repo()  # later\n";
    let at = |line, name| bound_at(Kind::Python, text, line, name);
    let call = |c: &str| Value::Call(c.into());
    // The innermost function binding the name hides the one around it.
    assert_eq!(at(6, "repo"), [(5, call("UserRepository"))]);
    // One that does not bind it reads the enclosing function's.
    assert_eq!(at(6, "user_id"), [(1, ty("int"))]);
    // The whole function counts, the lines after the cursor too.
    assert_eq!(
        at(8, "repo"),
        [(2, call("AuditLog")), (9, call("make_repo"))]
    );
    assert_eq!(at(6, "user_id"), [(1, ty("int"))]);
}

#[test]
fn ts_bindings_follow_the_blocks_around_the_cursor() {
    let text = r#"import { AuditLog, UserRepository } from "./repos";

const shared = new AuditLog();

export class UserService {
  private audit = new AuditLog();

  constructor(
    private repo: UserRepository,
  ) {
    this.repo.findUser(1);
  }

  async remove(id: number, cache = new Cache()): Promise<void> {
    const user: User = this.repo.findUser(id);
    const made = await createRepo();
    for (const item of items) {
      item.go();
    }
    const { a, b } = user;
    items.forEach((x: Item) => x.go());
    const handler = function () {
      this.go();
    };
    const later = () => {
      this.audit.deleteUser(id);
    };
    shared.go(made, a);
  }
}

export function cleanup(id: number): void {
  const repo = new AuditLog();
  const purge = async () => {
    const repo = new UserRepository();
    await repo.deleteUser(id);
  };
  repo.deleteUser(id);
}
"#;
    let at = |line, name| bound_at(Kind::TsJs, text, line, name);
    let new = |t: &str| Value::New(t.into());
    // `this` passes a constructor over several lines, an arrow and a method.
    assert_eq!(at(11, "this"), [(5, Value::Class(5))]);
    assert_eq!(at(26, "this"), [(5, Value::Class(5))]);
    assert_eq!(at(23, "this"), [(22, Value::Unknown)]);
    assert_eq!(at(11, "repo"), [(8, ty("UserRepository"))]);
    assert_eq!(at(15, "id"), [(14, ty("number"))]);
    assert_eq!(at(15, "cache"), [(14, new("Cache"))]);
    assert_eq!(at(16, "user"), [(15, ty("User"))]);
    assert_eq!(at(28, "made"), [(16, Value::Call("createRepo".into()))]);
    assert_eq!(at(28, "shared"), [(3, new("AuditLog"))]);
    assert_eq!(at(18, "item"), [(17, Value::Element("items".into()))]);
    // A destructuring hands on a field of what stands on its right (#100).
    assert_eq!(
        at(28, "a"),
        [(20, Value::Field(vec!["user".into()], "a".into()))]
    );
    assert_eq!(at(21, "x"), [(21, ty("Item"))]);
    // The innermost block around the cursor that declares the name; the inner `repo` is
    // gone below its arrow.
    assert_eq!(at(36, "repo"), [(35, new("UserRepository"))]);
    assert_eq!(at(38, "repo"), [(33, new("AuditLog"))]);
}

#[test]
fn what_a_header_binds_for_another_body_hides_nothing() {
    let new = |t: &str| Value::New(t.into());
    // A loop and a one-name arrow inside a callback on the line that opens a literal or
    // another callback: the cursor below is in neither, and the outer `repo` still counts.
    let ts = "const repo = new AuditLog();\nrun(() => { for (const repo of repos) { use(repo); } }, {\n  done: repo.x(),\n});\nrepos.map(repo => repo.id).forEach((id) => {\n  repo.y(id);\n});\nrepos.forEach(repo => {\n  repo.z();\n});\n";
    let at = |line| bound_at(Kind::TsJs, ts, line, "repo");
    assert_eq!(
        at(3),
        [(2, Value::Element("repos".into())), (1, new("AuditLog"))]
    );
    assert_eq!(at(6), [(5, Value::Unknown), (1, new("AuditLog"))]);
    // The arrow whose body the line opens is the cursor's own.
    assert_eq!(at(9), [(8, Value::Unknown)]);
    // A declaration inside a raw string over several lines is none.
    let go = "func f() {\n\trepo := NewAudit()\n\tif ok {\n\t\tconst doc = `\n\t\trepo := NewRepo()\n\t\t`\n\t\trepo.Go(doc)\n\t}\n}\n";
    assert_eq!(
        bound_at(Kind::Go, go, 7, "repo"),
        [(2, Value::Call("NewAudit".into()))]
    );
}

#[test]
fn go_bindings_cover_receivers_parameters_and_short_declarations() {
    let text = "package main

var shared = &AuditLog{}

func (s *UserService) Remove(id int, a, b *Repo) (n int, err error) {
\tuser := s.repo.FindUser(id)
\tmade, err := NewRepo()
\tvar typed store.Session
\tvar built = Repo{ID: 1}
\tfresh := new(Repo)
\tfor _, item := range items {
\t\titem.Go()
\t}
\tif r, ok := x.(*Repo); ok {
\t\tr.Go()
\t}
\tpurge := func(repo *UserRepository) {
\t\trepo.DeleteUser(id)
\t}
\t_, other := pair()
\tshared.Go(user, made, typed, built, fresh, other, purge)
}
";
    let at = |line, name| bound_at(Kind::Go, text, line, name);
    let new = |t: &str| Value::New(t.into());
    assert_eq!(at(6, "s"), [(5, ty("*UserService"))]);
    assert_eq!(at(21, "a"), [(5, ty("*Repo"))]);
    assert_eq!(at(21, "b"), [(5, ty("*Repo"))]);
    assert_eq!(at(21, "n"), [(5, ty("int"))]);
    assert_eq!(at(21, "made"), [(7, Value::Call("NewRepo".into()))]);
    // The second name of a `:=` is not the call's first result.
    assert_eq!(at(21, "err"), [(7, Value::Unknown), (5, ty("error"))]);
    assert_eq!(at(21, "typed"), [(8, ty("store.Session"))]);
    assert_eq!(at(21, "built"), [(9, new("Repo"))]);
    assert_eq!(at(21, "fresh"), [(10, new("Repo"))]);
    assert_eq!(at(21, "shared"), [(3, new("AuditLog"))]);
    assert_eq!(at(12, "item"), [(11, Value::Element("items".into()))]);
    assert_eq!(at(15, "r"), [(14, Value::Unknown)]);
    assert_eq!(at(18, "repo"), [(17, ty("*UserRepository"))]);
    assert_eq!(at(21, "other"), [(20, Value::Unknown)]);
}

fn fields(kind: Kind, text: &str, decl: usize, name: &str) -> Vec<(usize, Value)> {
    field_bindings(kind, text, decl, name)
        .into_iter()
        .map(|b| (b.line, b.value))
        .collect()
}

#[test]
fn python_fields_come_from_the_class_body_and_self_in_its_methods() {
    let text = "class Service(Base, metaclass=Meta):
    audit: AuditLog
    cache = Cache()

    def __init__(self, repo: UserRepository) -> None:
        self.repo = repo
        self.store: Store | None = None
        self.a, self.b = 1, 2
        log(self.repo, self.audit)

    class Inner:
        def __init__(self):
            self.repo = Other()

    def reset(self):
        self.repo = make_repo()
";
    let at = |name| fields(Kind::Python, text, 1, name);
    // An argument of a call is no binding.
    assert_eq!(
        at("repo"),
        [
            (6, Value::Name("repo".into())),
            (16, Value::Call("make_repo".into()))
        ]
    );
    assert_eq!(at("audit"), [(2, ty("AuditLog"))]);
    assert_eq!(at("cache"), [(3, Value::Call("Cache".into()))]);
    assert_eq!(at("store"), [(7, ty("Store | None"))]);
    assert_eq!(at("a"), [(8, Value::Unknown)]);
    assert_eq!(at("b"), [(8, Value::Unknown)]);
    assert_eq!(bases(Kind::Python, text, 1), ["Base"]);
}

#[test]
fn a_python_property_is_a_field_of_its_declared_return_type() {
    let text = "class Registry(Base):
    @property
    def users(self) -> UserRepository:
        return UserRepository()

    @users.setter
    def users(self, value: Other) -> None:
        self._users = value

    @cached_property  # built once
    def audit(
        self,
    ) -> \"AuditLog\":
        return AuditLog()

    @functools.cached_property
    @override
    def jobs(self) -> Jobs: ...

    @property
    def mixins(self):
        return HttpRepo()

    def plain(self) -> Plain: ...

    class Inner:
        @property
        def inner(self) -> Inner: ...

    def make(self):
        @property
        def nested(self) -> Nested: ...
";
    let at = |name| fields(Kind::Python, text, 1, name);
    // The setter declares no type of its own.
    assert_eq!(at("users"), [(3, ty("UserRepository"))]);
    assert_eq!(at("audit"), [(11, ty("\"AuditLog\""))]);
    assert_eq!(at("jobs"), [(18, ty("Jobs"))]);
    assert_eq!(at("mixins"), [(21, Value::Unknown)]);
    assert_eq!(at("plain"), []);
    assert_eq!(at("inner"), []);
    // A property nested in a method is no field of the class.
    assert_eq!(at("nested"), []);
}

#[test]
fn a_ts_getter_is_a_field_of_its_declared_type() {
    let text = "export abstract class Registry {
  get users(): UserRepository {
    return this.cache.users;
  }
  set users(value: Other) {
    this.cache.users = value;
  }
  public static get audit(): AuditLog | null { return null; }
  protected abstract get jobs(): Jobs;
  get mixins() {
    return new HttpRepo();
  }
  make() {
    return {
      get nested(): Other { return x; },
    };
  }
}
";
    let at = |name| fields(Kind::TsJs, text, 1, name);
    assert_eq!(at("users"), [(2, ty("UserRepository"))]);
    assert_eq!(at("audit"), [(8, ty("AuditLog | null"))]);
    assert_eq!(at("jobs"), [(9, ty("Jobs"))]);
    assert_eq!(at("mixins"), [(10, Value::Unknown)]);
    // An object literal's getter inside a method is no field of the class.
    assert_eq!(at("nested"), []);
}

#[test]
fn ts_fields_come_from_members_constructor_parameters_and_this() {
    let text = "export class UserService extends Base<Repo> implements Service {
  private readonly audit = new AuditLog();
  #root: Node;
  store?: Store | null;

  constructor(
    private repo: UserRepository,
    notifier: Notifier,
  ) {
    super();
    this.notifier = notifier;
    this.#root = new Node();
  }

  audit2(): void {}
}

export interface Store extends Reader, Writer<T> {
  readonly db: Database;
  send(text: string): void;
}
";
    let at = |decl, name| fields(Kind::TsJs, text, decl, name);
    assert_eq!(at(1, "audit"), [(2, Value::New("AuditLog".into()))]);
    assert_eq!(
        at(1, "#root"),
        [(3, ty("Node")), (12, Value::New("Node".into()))]
    );
    assert_eq!(at(1, "store"), [(4, ty("Store | null"))]);
    assert_eq!(at(1, "repo"), [(7, ty("UserRepository"))]);
    // A parameter without a modifier is no field; the assignment is.
    assert_eq!(at(1, "notifier"), [(11, Value::Name("notifier".into()))]);
    assert_eq!(at(18, "db"), [(19, ty("Database"))]);
    assert_eq!(at(18, "send"), []);
    assert_eq!(bases(Kind::TsJs, text, 1), ["Base<Repo>"]);
    assert_eq!(bases(Kind::TsJs, text, 18), ["Reader", "Writer<T>"]);
}

#[test]
fn go_fields_are_struct_fields_and_embedded_types() {
    let text = "type Context struct {
\t*Engine
\tsync.Mutex
\trepo, audit *UserRepository
\thandlers HandlersChain `json:\"-\"` // the chain

\tinner struct {
\t\trepo Other
\t}
}

type Store interface {
\tReader
\tGet(key string) string
}
";
    let at = |name| fields(Kind::Go, text, 1, name);
    assert_eq!(at("repo"), [(4, ty("*UserRepository"))]);
    assert_eq!(at("audit"), [(4, ty("*UserRepository"))]);
    assert_eq!(at("handlers"), [(5, ty("HandlersChain"))]);
    assert_eq!(at("Engine"), [(2, ty("*Engine"))]);
    assert_eq!(at("Mutex"), [(3, ty("sync.Mutex"))]);
    assert_eq!(fields(Kind::Go, text, 12, "Reader"), []);
    assert_eq!(bases(Kind::Go, text, 1), ["Engine", "sync.Mutex"]);
    assert_eq!(bases(Kind::Go, text, 12), ["Reader"]);
}

/// #68 step 6: the forms the implementations of a member are found through.
const PY_IMPLS: &str = "class Notifier(Protocol):
    def send(self, text: str) -> None: ...


class EmailNotifier(Notifier):
    def send(
        self,
        text: str,
    ) -> None:
        def inner() -> None:
            pass


class Quiet(Notifier):
    pass


def send(text: str) -> None:
    pass
";

const TS_IMPLS: &str = "export interface Notifier {
  send(text: string): void;
}

export abstract class Base<T> extends Other implements Notifier, Logger<T> {
  send(text: string): void {
    const inner = () => {};
  }
}

export class Quiet extends Base {}

export class Wrapped
  extends Base
  implements Notifier
{
  send(text: string): void {}
}
";

const GO_IMPLS: &str = "type Notifier interface {
\tSend(text string)
}

func (e *EmailNotifier) Send(text string) {
}

func (b Batch) Send(text string, retries int) {
}
";

#[test]
fn owner_decl_names_the_type_a_member_is_written_in() {
    let py = |line| owner_decl(Kind::Python, PY_IMPLS, line);
    assert_eq!(py(2), Some(1), "a protocol's signature");
    assert_eq!(py(6), Some(5), "a method");
    assert_eq!(py(10), None, "a function nested in a method is no member");
    assert_eq!(py(18), None, "a function at the top of the module");
    let ts = |line| owner_decl(Kind::TsJs, TS_IMPLS, line);
    assert_eq!(ts(2), Some(1), "an interface signature");
    assert_eq!(ts(6), Some(5));
    assert_eq!(ts(7), None);
    assert_eq!(ts(17), Some(13), "past the brace of a wrapped header");
    let go = |line| owner_decl(Kind::Go, GO_IMPLS, line);
    assert_eq!(go(2), Some(1), "an interface's method line");
    assert_eq!(go(5), None, "a method stands beside its type");
}

#[test]
fn interfaces_read_what_a_typescript_class_implements() {
    assert_eq!(
        interfaces(Kind::TsJs, TS_IMPLS, 5),
        ["Notifier", "Logger<T>"]
    );
    assert_eq!(bases(Kind::TsJs, TS_IMPLS, 5), ["Other"], "extends alone");
    assert_eq!(interfaces(Kind::TsJs, TS_IMPLS, 11), [] as [String; 0]);
    // A wrapped header carries each clause on a line of its own.
    assert_eq!(bases(Kind::TsJs, TS_IMPLS, 14), ["Base"]);
    assert_eq!(interfaces(Kind::TsJs, TS_IMPLS, 15), ["Notifier"]);
    assert_eq!(interfaces(Kind::Python, PY_IMPLS, 5), [] as [String; 0]);
    assert_eq!(bases(Kind::Python, PY_IMPLS, 5), ["Notifier"]);
}

#[test]
fn subtype_patterns_match_a_header_that_names_a_base() {
    let names = ["Notifier".to_owned(), "Base".to_owned()];
    let re = |kind| Regex::new(&subtype_patterns(kind, &names).unwrap()).unwrap();
    let py = re(Kind::Python);
    assert!(py.is_match("class EmailNotifier(Notifier):"));
    assert!(py.is_match("class X(Generic[T], Base):"));
    assert!(!py.is_match("class Notifier(Protocol):"), "its own header");
    assert!(!py.is_match("    notifier: Notifier"));
    let ts = re(Kind::TsJs);
    assert!(ts.is_match("export class EmailNotifier implements Notifier {"));
    assert!(ts.is_match("export interface Admin extends Notifier {"));
    assert!(ts.is_match("class X extends Base<T> implements Other {"));
    assert!(ts.is_match("    extends Base"), "a wrapped header's clause");
    assert!(!ts.is_match("export interface Notifier {"));
    assert!(!ts.is_match("export class NotifierFactory {"));
    assert_eq!(
        subtype_patterns(Kind::Go, &names),
        None,
        "implicit interfaces"
    );
}

#[test]
fn member_decl_finds_only_what_the_type_declares_itself() {
    assert_eq!(member_decl(Kind::Python, PY_IMPLS, 1, "send"), Some(2));
    assert_eq!(member_decl(Kind::Python, PY_IMPLS, 5, "send"), Some(6));
    assert_eq!(member_decl(Kind::Python, PY_IMPLS, 5, "inner"), None);
    assert_eq!(
        member_decl(Kind::Python, PY_IMPLS, 14, "send"),
        None,
        "inherits it"
    );
    assert_eq!(member_decl(Kind::TsJs, TS_IMPLS, 1, "send"), Some(2));
    assert_eq!(member_decl(Kind::TsJs, TS_IMPLS, 5, "send"), Some(6));
    assert_eq!(member_decl(Kind::TsJs, TS_IMPLS, 11, "send"), None);
    assert_eq!(member_decl(Kind::TsJs, TS_IMPLS, 13, "send"), Some(17));
}

#[test]
fn type_decl_at_walks_up_a_header_wrapped_over_several_lines() {
    let at = |line| type_decl_at(Kind::TsJs, TS_IMPLS, line);
    assert_eq!(at(13), Some(13), "the header itself");
    assert_eq!(at(14), Some(13), "`extends Base` on its own line");
    assert_eq!(at(15), Some(13), "`implements Notifier` on its own");
    assert_eq!(at(16), Some(13), "the brace below them");
    assert_eq!(at(8), None, "a line that ends a block declares nothing");
    assert_eq!(
        qualified(Kind::TsJs, TS_IMPLS, 17, "send"),
        Some("Wrapped.send".into()),
        "the member of a wrapped class is named by it"
    );
}

#[test]
fn params_count_what_a_declaration_takes() {
    assert_eq!(params(Kind::Python, PY_IMPLS, 2), Some(2));
    assert_eq!(
        params(Kind::Python, PY_IMPLS, 6),
        Some(2),
        "over three lines"
    );
    assert_eq!(params(Kind::Python, PY_IMPLS, 18), Some(1));
    assert_eq!(params(Kind::TsJs, TS_IMPLS, 2), Some(1));
    assert_eq!(
        params(Kind::Go, GO_IMPLS, 2),
        Some(1),
        "an interface's line"
    );
    assert_eq!(params(Kind::Go, GO_IMPLS, 5), Some(1), "past the receiver");
    assert_eq!(params(Kind::Go, GO_IMPLS, 8), Some(2));
    assert_eq!(params(Kind::Python, PY_IMPLS, 1), Some(1), "a class header");
}

#[test]
fn type_name_reads_the_name_a_declaration_gives() {
    let name = |kind, line| type_name(kind, line);
    assert_eq!(name(Kind::Python, "class Foo(Base):"), Some("Foo".into()));
    assert_eq!(name(Kind::Python, "    def send(self):"), None);
    assert_eq!(
        name(Kind::TsJs, "export abstract class Foo<T> extends Bar {"),
        Some("Foo".into())
    );
    assert_eq!(
        name(Kind::TsJs, "export interface Foo {"),
        Some("Foo".into())
    );
    assert_eq!(name(Kind::TsJs, "  send(text: string): void;"), None);
    assert_eq!(name(Kind::Go, "type Foo struct{}"), Some("Foo".into()));
    assert_eq!(name(Kind::Go, "func (f Foo) Send() {"), None);
}

#[test]
fn returns_read_the_declared_type_or_what_typescript_constructs() {
    let py = "def make_repo() -> UserRepository:\n    return UserRepository()\n\nasync def connect(\n    url: str,\n) -> \"Session\":\n    ...\n\ndef untyped():\n    return Repo()\n\ndef stub() -> Repo: ...\n\ndef documented() -> Annotated[Repo, \"doc: x\"]: ...\n";
    assert_eq!(returns(Kind::Python, py, 1), Some(ty("UserRepository")));
    assert_eq!(returns(Kind::Python, py, 4), Some(ty("\"Session\"")));
    // No annotation: what every `return` constructs (#100).
    assert_eq!(
        returns(Kind::Python, py, 9),
        Some(Value::New("Repo".into()))
    );
    assert_eq!(returns(Kind::Python, py, 12), Some(ty("Repo")));
    // A `: ` inside a string is not the end of the annotation.
    assert_eq!(
        returns(Kind::Python, py, 14),
        Some(ty("Annotated[Repo, \"doc: x\"]"))
    );
    let ts = "export function makeRepo(): UserRepository {
  return new UserRepository();
}
export async function load(id: number): Promise<Repo> {
  return fetchRepo(id);
}
export function makeAudit() {
  if (x) {
    return new AuditLog();
  }
  return new AuditLog();
}
export const createRepo = (): Repo => new Repo();
export const inferred = () => new Repo();
export function mixed() {
  if (x) {
    return new AuditLog();
  }
  return new UserRepository();
}
export default function connect(): Session {
  return openSession();
}
";
    let new = |t: &str| Some(Value::New(t.into()));
    assert_eq!(returns(Kind::TsJs, ts, 1), Some(ty("UserRepository")));
    assert_eq!(returns(Kind::TsJs, ts, 4), Some(ty("Promise<Repo>")));
    assert_eq!(returns(Kind::TsJs, ts, 7), new("AuditLog"));
    assert_eq!(returns(Kind::TsJs, ts, 13), Some(ty("Repo")));
    assert_eq!(returns(Kind::TsJs, ts, 14), new("Repo"));
    assert_eq!(returns(Kind::TsJs, ts, 15), None);
    assert_eq!(returns(Kind::TsJs, ts, 21), Some(ty("Session")));
    let go = "func NewRepo() *UserRepository {
\treturn &UserRepository{}
}
func NewAudit(url string) (AuditLog, error) {
\treturn AuditLog{}, nil
}
func Open() (s *Session, err error) {
\treturn
}
func Close() {
}
";
    assert_eq!(returns(Kind::Go, go, 1), Some(ty("*UserRepository")));
    assert_eq!(returns(Kind::Go, go, 4), Some(ty("AuditLog")));
    assert_eq!(returns(Kind::Go, go, 7), Some(ty("*Session")));
    assert_eq!(returns(Kind::Go, go, 10), None);
}

#[test]
fn returns_read_methods_and_what_an_unannotated_python_def_constructs() {
    let new = |t: &str| Some(Value::New(t.into()));
    let py = r#"class Depot:
    def repo(self) -> Repo:
        return self.people

    def trail(self):
        """Doc.

        return Wrong()
        """
        def inner():
            return Other()

        if self.people:
            return Trail(
                self,
            )
        return Trail()  # again

    def either(self):
        if self.people:
            return Trail()
        return Repo()

    def maybe(self):
        if self.people:
            return Trail()
        return

    def stream(self):
        yield Trail()
        return Trail()

    @cache
    def cached(self):
        return Trail()

    def handed_on(self):
        return self.people
"#;
    assert_eq!(returns(Kind::Python, py, 2), Some(ty("Repo")));
    assert_eq!(returns(Kind::Python, py, 5), new("Trail"));
    assert_eq!(returns(Kind::Python, py, 19), None);
    assert_eq!(returns(Kind::Python, py, 24), None);
    assert_eq!(returns(Kind::Python, py, 29), None);
    assert_eq!(returns(Kind::Python, py, 34), None);
    assert_eq!(returns(Kind::Python, py, 37), None);
    let ts = "export class Depot {\n  async repo<T>(id: T): Promise<Repo> {\n    return load(id);\n  }\n  private static trail() {\n    return new Trail();\n  }\n  find = (id: number): Repo => load(id);\n}\ninterface Source {\n  source(): Repo;\n  maybe?(): Repo\n}\n";
    assert_eq!(returns(Kind::TsJs, ts, 2), Some(ty("Promise<Repo>")));
    assert_eq!(returns(Kind::TsJs, ts, 5), new("Trail"));
    // A property holding a function is not read.
    assert_eq!(returns(Kind::TsJs, ts, 8), None);
    assert_eq!(returns(Kind::TsJs, ts, 11), Some(ty("Repo")));
    assert_eq!(returns(Kind::TsJs, ts, 12), Some(ty("Repo")));
    let go = "func (d *Depot) Repo(id int) *Repo {\n\treturn d.people\n}\nfunc (d Depot) Trail() (Trail, error) {\n\treturn Trail{}, nil\n}\ntype Source interface {\n\tSource() *Repo\n\tClose()\n}\n";
    assert_eq!(returns(Kind::Go, go, 1), Some(ty("*Repo")));
    assert_eq!(returns(Kind::Go, go, 4), Some(ty("Trail")));
    assert_eq!(returns(Kind::Go, go, 8), Some(ty("*Repo")));
    assert_eq!(returns(Kind::Go, go, 9), None);
}

/// Found by the hand pass of #100 in mealie: a string continued with a backslash swallowed
/// the end of its line, the answer came out a line short, and `d` anywhere in such a Python
/// file indexed past it and crashed.
#[test]
fn a_backslash_at_the_end_of_a_line_keeps_the_line_count() {
    let py = "log(\"a \\\n    b\")\nledger = A()\nledger.go()";
    assert_eq!(literal_lines(Kind::Python, py), [false; 4]);
    let found = bindings(Kind::Python, py, 4, "ledger");
    assert_eq!(found.len(), 1);
}

#[test]
fn a_python_line_is_cut_into_its_simple_statements() {
    let cases: [(&str, &[&str]); 10] = [
        ("ledger = A()", &["ledger = A()"]),
        ("if x: ledger = A()", &["ledger = A()"]),
        ("else: a = 1; b = 2", &["a = 1", "b = 2"]),
        ("async with open(p) as f: data = f", &["data = f"]),
        // A `:` in brackets, in a string and of `:=` ends no header.
        ("if d[1:2] == \"a:b\": x = 1", &["x = 1"]),
        ("if m := find(): x = m", &["x = m"]),
        ("while True:", &[]),
        ("case Repo(): x = 1", &["x = 1"]),
        // No header: an annotation's `:` cuts nothing, and neither does a `;` in a string.
        ("iffy: int = 1", &["iffy: int = 1"]),
        ("x = \"a;b\"", &["x = \"a;b\""]),
    ];
    for (line, want) in cases {
        assert_eq!(python_statements(line, false), want, "{line}");
    }
    // The last line of a wrapped header, whatever the line above it ends in.
    for continues in [false, true] {
        let got = python_statements("flag): x = 1; y = 2", continues);
        assert_eq!(got, ["x = 1", "y = 2"]);
    }
    let continued: [(&str, &[&str]); 3] = [
        ("b=2, x = 1", &[]),
        ("if c else d)", &[]),
        ("key=lambda v: v)", &[]),
    ];
    for (line, want) in continued {
        assert_eq!(python_statements(line, true), want, "{line}");
    }
}

#[test]
fn a_cast_is_read_as_the_type_it_writes() {
    let v = |kind, e| value_of(kind, e);
    let pycast = |callee: &str, t: &str| Value::Cast(callee.into(), t.into());
    assert_eq!(v(Kind::Python, "cast(Repo, row)"), pycast("cast", "Repo"));
    assert_eq!(
        v(Kind::Python, "typing.cast(\"models.Repo\", rows[0])"),
        pycast("typing.cast", "\"models.Repo\"")
    );
    assert_eq!(v(Kind::Python, "cast(Repo, row).other"), Value::Unknown);
    assert_eq!(
        v(Kind::Python, "recast(Repo, row)"),
        Value::Call("recast".into())
    );
    assert_eq!(v(Kind::TsJs, "row as Repo;"), ty("Repo"));
    assert_eq!(v(Kind::TsJs, "load(id) as unknown as Repo"), ty("Repo"));
    assert_eq!(v(Kind::TsJs, "{ a: 1 } as const"), ty("const"));
    assert_eq!(v(Kind::TsJs, "await load(a, b) as Repo"), ty("Repo"));
    // `as` takes the operand next to it, not the whole expression.
    assert_eq!(v(Kind::TsJs, "ok ? a : b as Repo"), Value::Unknown);
    assert_eq!(v(Kind::TsJs, "a ?? b as Repo"), Value::Unknown);
    assert_eq!(v(Kind::TsJs, "() => row as Repo"), Value::Unknown);
    assert_eq!(
        v(Kind::TsJs, "check(x as Foo) ? other : found as Repo"),
        Value::Unknown
    );
    assert_eq!(v(Kind::TsJs, "new Wrapper(x as Foo) as Repo"), ty("Repo"));
    // An `as` inside brackets or a string casts something else.
    assert_eq!(
        v(Kind::TsJs, "load(row as Repo)"),
        Value::Call("load".into())
    );
    assert_eq!(
        v(Kind::TsJs, "pick(\"x as Repo\")"),
        Value::Call("pick".into())
    );
    assert_eq!(v(Kind::Go, "i.(*Repo)"), ty("*Repo"));
    assert_eq!(v(Kind::Go, "ctx.Value.(models.Repo)"), ty("models.Repo"));
    assert_eq!(v(Kind::Go, "i.(*Repo).Owner"), Value::Unknown);

    let go = "func f(x any) {\n\tswitch v := x.(type) {\n\tcase *Repo:\n\t\tv.Go()\n\tcase A, B:\n\t\tv.Go()\n\tdefault:\n\t\tv.Go()\n\t}\n\tswitch v := x.(type) {\n\tcase *Audit:\n\t\tswitch x {\n\t\tcase 1:\n\t\t\tv.Go()\n\t\t}\n\t}\n\tswitch v := pick(); v {\n\tcase 1:\n\t\tv.Go()\n\t}\n}\n";
    let at = |line| bound_at(Kind::Go, go, line, "v");
    assert_eq!(at(4), [(2, ty("*Repo"))]);
    assert_eq!(at(6), [(2, Value::Unknown)]);
    assert_eq!(at(8), [(2, Value::Unknown)]);
    // Past a plain `switch` inside the arm, and not the closed type switch above.
    assert_eq!(at(14), [(10, ty("*Audit"))]);
    // No type switch: nothing the rules read binds `v` here.
    assert_eq!(at(19), []);
    // A `select` ends its `case` as a `switch` does: the closed type switch above it is not
    // read with the `case` of the `select`, and the parameter `v` stays what it is.
    let chans = "func f(v *Repo, x any, ch chan int) {\n\tswitch v := x.(type) {\n\tcase *Audit:\n\t\tv.Go()\n\t}\n\tselect {\n\tcase <-ch:\n\t\tv.Go()\n\t}\n}\n";
    assert_eq!(bound_at(Kind::Go, chans, 8, "v"), [(1, ty("*Repo"))]);
}

#[test]
fn a_chain_may_hang_off_the_call_that_starts_it() {
    let head = |kind, line: &str| {
        let at = line.rfind('.').unwrap() + 1;
        call_head(kind, line, at).map(|(label, value, fields)| (label, value, fields.join(".")))
    };
    let call = |label: &str, callee: &str, fields: &str| {
        Some((
            label.to_owned(),
            Value::Call(callee.into()),
            fields.to_owned(),
        ))
    };
    assert_eq!(
        head(Kind::Go, "\treturn pkg.New(x, y).Run()"),
        call("pkg.New()", "pkg.New", "")
    );
    assert_eq!(
        head(
            Kind::Python,
            "    await make_uow(\")\").users.delete_user(1)"
        ),
        call("make_uow()", "make_uow", "users")
    );
    assert_eq!(
        head(Kind::Python, "x = self.repos.users.get_one(f(a), b).name"),
        call("self.repos.users.get_one()", "self.repos.users.get_one", "")
    );
    assert_eq!(
        head(Kind::TsJs, "  void new Depot(a).people.deleteUser(1);"),
        Some((
            "new Depot()".to_owned(),
            Value::New("Depot".into()),
            "people".to_owned()
        ))
    );
    let cast = |written: &str, t: &str| Some((written.to_owned(), ty(t), String::new()));
    assert_eq!(
        head(Kind::TsJs, "  void (found as Repo).find()"),
        cast("found as Repo", "Repo")
    );
    assert_eq!(
        head(Kind::Go, "\tfound.(*Repo).Find()"),
        cast("found.(*Repo)", "*Repo")
    );
    assert_eq!(
        head(Kind::Python, "    cast(Repo, found).find()"),
        Some((
            "cast(Repo, found)".to_owned(),
            Value::Cast("cast".into(), "Repo".into()),
            String::new()
        ))
    );
    // Brackets around a call are read through, as an `await` in front of one is.
    assert_eq!(
        head(Kind::TsJs, "  (await load()).find()"),
        call("load()", "load", "")
    );
    // A call of a call, an index, a generic call, a condition, a plain name.
    assert_eq!(head(Kind::Go, "\tOpen().Repo().Delete()"), None);
    assert_eq!(head(Kind::Python, "    make()[0].delete()"), None);
    assert_eq!(head(Kind::Python, "    items[0].load().delete()"), None);
    assert_eq!(head(Kind::TsJs, "  load<Repo>(id).find()"), None);
    assert_eq!(head(Kind::TsJs, "  if (ok).find()"), None);
    assert_eq!(head(Kind::Python, "    repo.find()"), None);
    // A callee or a field with a character outside ASCII: no name to read, and no slice inside
    // the character either (#150).
    assert_eq!(head(Kind::TsJs, "  void café().word"), None);
    assert_eq!(head(Kind::Python, "    café.users.delete(1)"), None);
}

#[test]
fn a_written_type_comes_down_to_one_name() {
    let path = |kind, t| type_path(kind, t).map(|p| p.join("."));
    let some = |s: &str| Some(s.to_owned());
    for (t, want) in [
        ("UserRepository", some("UserRepository")),
        ("\"Session\"", some("Session")),
        ("Optional[Repo]", some("Repo")),
        ("Repo | None", some("Repo")),
        ("Annotated[Repo, Depends(get_repo)]", some("Repo")),
        ("repos.UserRepository", some("repos.UserRepository")),
        ("list[Repo]", some("list")),
        ("Repo | Other", None),
    ] {
        assert_eq!(path(Kind::Python, t), want, "{t}");
    }
    for (t, want) in [
        ("Repo<User>", some("Repo")),
        ("Repo | null | undefined", some("Repo")),
        ("Repo[]", None),
        ("(a: A) => B", None),
        ("{ a: number }", None),
    ] {
        assert_eq!(path(Kind::TsJs, t), want, "{t}");
    }
    for (t, want) in [
        ("*Repo", some("Repo")),
        ("store.Session", some("store.Session")),
        ("Set[T]", some("Set")),
        ("[]Repo", None),
        ("map[string]Repo", None),
    ] {
        assert_eq!(path(Kind::Go, t), want, "{t}");
    }
}

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
            ("fs".into(), p(&["fs", "default"])),
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

#[test]
fn definition_scope_follows_the_kind() {
    let here = Path::new("infra/app/main.tf");
    assert!(in_def_scope(
        Kind::Terraform,
        here,
        Path::new("infra/app/vars.tf")
    ));
    assert!(!in_def_scope(
        Kind::Terraform,
        here,
        Path::new("infra/vpc/vars.tf")
    ));
    assert!(!in_def_scope(
        Kind::Terraform,
        here,
        Path::new("infra/app/notes.md")
    ));
    let compose = Path::new("compose.yml");
    assert!(in_def_scope(Kind::Yaml, compose, compose));
    assert!(!in_def_scope(Kind::Yaml, compose, Path::new("other.yml")));
    assert!(in_def_scope(
        Kind::Make,
        Path::new("Makefile"),
        Path::new("tests/rules.mk")
    ));
    assert!(!in_def_scope(
        Kind::Make,
        Path::new("Makefile"),
        Path::new("a.py")
    ));
    // `.tsx` finds its types in `.ts` and its helpers in `.js`.
    assert!(in_def_scope(
        Kind::TsJs,
        Path::new("ui/app.tsx"),
        Path::new("lib/types.ts")
    ));
    assert!(in_def_scope(
        Kind::TsJs,
        Path::new("ui/app.tsx"),
        Path::new("e.js")
    ));
    assert!(!in_def_scope(
        Kind::Rust,
        Path::new("src/main.rs"),
        Path::new("build.py")
    ));
    // A Kotlin file finds the Java class it calls, and the other way round.
    assert!(in_def_scope(
        Kind::Jvm,
        Path::new("app/src/App.kt"),
        Path::new("lib/src/Invoice.java")
    ));
    assert!(!in_def_scope(
        Kind::Jvm,
        Path::new("app/src/App.kt"),
        Path::new("lib/src/invoice.py")
    ));
    // A Rakefile finds the class it drives in the library it loads.
    assert!(in_def_scope(
        Kind::Ruby,
        Path::new("Rakefile"),
        Path::new("lib/invoice.rb")
    ));
    // A migration finds the table it alters in whatever file created it.
    assert!(in_def_scope(
        Kind::Sql,
        Path::new("migrations/002.sql"),
        Path::new("schema.ddl")
    ));
    assert!(!in_def_scope(
        Kind::Sql,
        Path::new("migrations/002.sql"),
        Path::new("notes.md")
    ));
}

#[test]
fn whole_word_excludes_longer_identifiers() {
    let (dir, files) = project("word");
    let py = files[..1].to_vec();
    assert_eq!(
        lines(&grep(&dir, &py, "total", true, false)),
        [("a.py".into(), 2)],
        "total_foobar is not the word `total`"
    );
    assert_eq!(lines(&grep(&dir, &py, "total", false, false)).len(), 3);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn smart_case_only_ignores_case_for_lowercase_patterns() {
    let (dir, files) = project("case");
    // Lowercase: `total`, `total_foobar` and Go's `Total`.
    assert_eq!(
        lines(&grep(&dir, &files, "total", false, true)),
        [
            ("a.py".into(), 2),
            ("a.py".into(), 11),
            ("a.py".into(), 12),
            ("b.go".into(), 5)
        ]
    );
    // One uppercase letter makes the whole pattern case-sensitive.
    assert_eq!(
        lines(&grep(&dir, &files, "Total", false, true)),
        [("b.go".into(), 5)]
    );
    // Without smart case a lowercase pattern stays case-sensitive too.
    assert_eq!(lines(&grep(&dir, &files, "invoice", false, false)), []);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn current_file_sorts_first_then_path_and_line() {
    let (dir, files) = project("sort");
    let hits = grep_project(
        &dir,
        &files,
        "Invoice",
        false,
        false,
        Some(Path::new("b.go")),
        None,
    )
    .unwrap();
    assert_eq!(
        lines(&hits),
        [
            ("b.go".into(), 3),
            ("b.go".into(), 5),
            ("b.go".into(), 7),
            ("a.py".into(), 1),
            ("a.py".into(), 7)
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Every pattern of [`LAST`], and the near-misses that carry the same letters but are code.
#[test]
fn tests_mocks_fixtures_and_generated_files_rank_last() {
    let here = Path::new("src/users/service.py");
    for (path, last) in [
        ("test/test_users.py", true),
        ("tests/users.py", true),
        ("src/__tests__/users.ts", true),
        ("spec/users_spec.rb", true),
        ("specs/users.rb", true),
        ("pkg/testdata/golden.go", true),
        ("tests/fixtures/users.json", true),
        ("src/__fixtures__/users.ts", true),
        ("src/mocks/store.ts", true),
        ("src/__mocks__/store.ts", true),
        ("vendor/github.com/x/y/y.go", true),
        ("third_party/x/y.py", true),
        ("src/test_users.py", true),
        ("src/conftest.py", true),
        ("pkg/users_test.go", true),
        ("src/users_spec.rb", true),
        ("src/users.test.ts", true),
        ("src/users.spec.ts", true),
        ("api/users_pb2.py", true),
        ("api/users_pb2_grpc.py", true),
        ("api/users.pb.go", true),
        ("api/users.gen.go", true),
        ("api/users.generated.ts", true),
        // The near-misses: the letters are there, the pattern is not.
        ("src/contest.rs", false),
        ("latest/users.py", false),
        ("src/testimonials.py", false),
        ("docs/spec.md", false),
        ("src/users/service.py", false),
        ("src/protest.go", false),
        ("src/specs.py", false),
    ] {
        let tier = rank(Path::new(path), Some(here), false).0;
        assert_eq!(tier == Tier::Tests, last, "{path} ranked {tier:?}");
    }
}

#[test]
fn a_declaration_comes_first_and_the_nearest_directory_next() {
    let here = Path::new("src/users/service.py");
    let paths = [
        "src/api/admin.py",
        "tests/test_service.py",
        "src/users/repo.py",
        "src/users/service.py",
        "src/users/admin/view.py",
        "src/users/repo.py",
    ];
    // The last row is the declaration; the open file is the one that equals `here`.
    let declaration = |p: &str, i: usize| p == "src/users/repo.py" && i == 5;
    let mut order: Vec<(usize, &str)> = paths.iter().copied().enumerate().collect();
    order.sort_by_key(|&(i, p)| (rank(Path::new(p), Some(here), declaration(p, i)), p, i));
    assert_eq!(
        order.iter().map(|&(_, p)| p).collect::<Vec<_>>(),
        [
            "src/users/repo.py",       // the declaration
            "src/users/service.py",    // the open file
            "src/users/repo.py",       // the same directory
            "src/users/admin/view.py", // one below
            "src/api/admin.py",        // one up and one down
            "tests/test_service.py",   // a test file, whatever its distance
        ]
    );
    // The open file is never demoted, even when it is a test file itself.
    let test = Path::new("tests/test_service.py");
    assert_eq!(rank(test, Some(test), false).0, Tier::Open);
    // `d` asks for the tier alone: every candidate of its own is a declaration.
    assert_eq!(rank(test, None, true).0, Tier::Tests);
    assert_eq!(rank(here, None, true).0, Tier::Declaration);
}

fn symbol(kind: Option<Kind>, line: &str) -> Option<String> {
    let (_, pattern) = SYMBOLS.iter().find(|(k, _)| *k == kind).unwrap();
    symbol_name(&Regex::new(pattern).unwrap(), line)
}

/// Every name `D` lists for `line` in a file of `kind`: each [`SYMBOLS`] row such a file is
/// read with, the all-language one included when [`shared_symbols`] takes it. A line listed
/// twice comes back twice.
fn listed(kind: Kind, line: &str) -> Vec<String> {
    SYMBOLS
        .iter()
        .filter(|(k, _)| *k == Some(kind) || (k.is_none() && shared_symbols(Some(kind))))
        .filter_map(|(_, p)| symbol_name(&Regex::new(p).unwrap(), line))
        .collect()
}

/// The one name `D` lists, and no second one.
fn one(kind: Kind, line: &str) -> Option<String> {
    let names = listed(kind, line);
    assert!(names.len() <= 1, "{line}: listed as {names:?}");
    names.into_iter().next()
}

#[test]
fn code_symbol_names() {
    for (line, name) in [
        ("def render(inv: Invoice) -> str:", Some("render")),
        ("    def total(self) -> int:", Some("total")),
        ("pub fn wrap_line(s: &str) {", Some("wrap_line")),
        ("pub(crate) struct Hit {", Some("Hit")),
        ("export async function parse(x) {", Some("parse")),
        ("export default class Foo {", Some("Foo")),
        ("    return total", None),
        ("class Invoice:", Some("Invoice")),
        ("func (i Invoice) Total() int {", Some("Total")),
        ("func main() {", Some("main")),
        ("type Invoice struct{}", Some("Invoice")),
        ("impl<T> Order<T> {", Some("Order")),
        (
            "pub(crate) const MAX_ORDERS: usize = 10;",
            Some("MAX_ORDERS"),
        ),
        ("static COUNT: u32 = 0;", Some("COUNT")),
        // Indented and bare, `const` and `static` are locals; exported, they are symbols.
        ("  const n = 1;", None),
        ("    static COUNT: u32 = 0;", None),
        ("  export const LIMIT = 10;", Some("LIMIT")),
        ("    pub const MAX: u32 = 1;", Some("MAX")),
        ("const fn zero() -> u32 {", Some("zero")),
        ("extern \"C\" fn c_call() {", Some("c_call")),
        ("macro_rules! order {", Some("order")),
        ("mod orders;", Some("orders")),
        ("pub union Bits {", Some("Bits")),
        (
            "export declare function load(id: string): void;",
            Some("load"),
        ),
        ("function* ids() {", Some("ids")),
        ("export namespace Orders {", Some("Orders")),
        ("export const parse = (s) => s;", Some("parse")),
        ("    render(o);", None),
        ("    return inv.render()", None),
        // A Go constant with a type or with several names, and a JavaScript name with a `$`:
        // the name is whatever follows the keyword, with nothing required after it.
        ("const Limit int = 10", Some("Limit")),
        ("const A, B = 1, 2", Some("A")),
        (
            "export const user$ = new BehaviorSubject(null);",
            Some("user"),
        ),
        // A dotted name is the first part: the namespace, not what is nested in it.
        ("export namespace Validation.Rules {", Some("Validation")),
        // Java, Kotlin and Ruby have rows of their own; their keywords are not here, so a Go
        // struct field and a line of prose are not declarations.
        ("\tmodule module.Version", None),
        ("module in general is to provide", None),
        ("module.exports = helpers;", None),
    ] {
        assert_eq!(symbol(None, line).as_deref(), name, "{line}");
    }
}

#[test]
fn shell_symbol_names() {
    let sh = |line| symbol(Some(Kind::Shell), line);
    assert_eq!(sh("build() {").as_deref(), Some("build"));
    assert_eq!(sh("run_all ( ) {").as_deref(), Some("run_all"));
    assert_eq!(
        sh("  helper() {").as_deref(),
        Some("helper"),
        "nested, like the rest"
    );
    // The `function` forms are listed by the generic pattern instead, so each shows once.
    assert_eq!(sh("function deploy {"), None);
    assert_eq!(sh("function check() {"), None);
    assert_eq!(symbol(None, "function deploy {").as_deref(), Some("deploy"));
    assert_eq!(symbol(None, "function check() {").as_deref(), Some("check"));
    assert_eq!(symbol(None, "build() {"), None);
    for not_a_function in ["  build \"$ROOT\"", "x=$(build)", "start)", "ROOT=/srv"] {
        assert_eq!(sh(not_a_function), None, "{not_a_function}");
    }
}

#[test]
fn sql_symbol_names() {
    let sql = |line| symbol(Some(Kind::Sql), line);
    assert_eq!(
        sql("CREATE TABLE public.orders (").as_deref(),
        Some("public.orders"),
        "the schema stays part of the name"
    );
    assert_eq!(
        sql("CREATE OR REPLACE FUNCTION total(o int)").as_deref(),
        Some("total")
    );
    assert_eq!(
        sql("CREATE UNIQUE INDEX orders_id_idx ON orders (id);").as_deref(),
        Some("orders_id_idx")
    );
    assert_eq!(
        sql("create materialized view daily_totals as").as_deref(),
        Some("daily_totals")
    );
    assert_eq!(
        sql("CREATE TABLE IF NOT EXISTS billing.invoices (").as_deref(),
        Some("billing.invoices")
    );
    assert_eq!(
        sql(r#"CREATE TABLE "user" ("#).as_deref(),
        Some(r#""user""#)
    );
    assert_eq!(
        sql("CREATE TYPE mood AS ENUM ('ok');").as_deref(),
        Some("mood")
    );
    for not_a_definition in [
        "SELECT * FROM orders;",
        "ALTER TABLE orders ADD COLUMN x int;",
        "DROP TABLE orders;",
        "INSERT INTO orders VALUES (1);",
        "CREATE TABLESPACE fast LOCATION '/x';",
    ] {
        assert_eq!(sql(not_a_definition), None, "{not_a_definition}");
    }
    // A CTE is a query's own scaffolding, not a project symbol.
    assert_eq!(sql("WITH recent AS ("), None);
    // The all-language pattern must not list SQL lines a second time: its keywords are
    // matched case-sensitively at the start of the line.
    for line in [
        "CREATE TYPE mood AS ENUM ('ok');",
        "create type mood as enum ('ok');",
        "CREATE TABLE public.orders (",
    ] {
        assert_eq!(symbol(None, line), None, "{line}");
    }
}

#[test]
fn jvm_symbol_names() {
    let jvm = |line| one(Kind::Jvm, line);
    // What either language declares with a keyword, behind annotations and modifiers.
    for (line, name) in [
        ("public final class Invoice {", Some("Invoice")),
        ("interface Store {", Some("Store")),
        ("record Point(int x, int y) {}", Some("Point")),
        ("enum Status {", Some("Status")),
        ("public @interface Json {", Some("Json")),
        ("data class Order(val id: String)", Some("Order")),
        ("enum class Status {", Some("Status")),
        ("annotation class Json", Some("Json")),
        ("    suspend fun load(id: String): Order {", Some("load")),
        ("@Test fun parsesInvoice() {", Some("parsesInvoice")),
        ("external fun nativeInit()", Some("nativeInit")),
        ("object Registry {", Some("Registry")),
        ("typealias Rows = List<Order>", Some("Rows")),
        ("fun interface Handler {", Some("Handler")),
        // An extension function is listed under its own name, past the receiver.
        ("fun String.slug(): String = lowercase()", Some("slug")),
        ("fun <T> List<T>.second(): T = this[1]", Some("second")),
        (
            "fun String?.orEmpty(): String = this ?: \"\"",
            Some("orEmpty"),
        ),
        // A method: the return type before the name is what tells it from a call.
        ("    public int total() {", Some("total")),
        (
            "    static Map<String, Integer> compute(Map<String, Integer> rows) {",
            Some("compute"),
        ),
        (
            "    Map<String, List<Integer>> group(List<Integer> xs) {",
            Some("group"),
        ),
        ("    void save(Invoice inv);", Some("save")),
        (
            "    @Override public static <T> List<T> of(T one) {",
            Some("of"),
        ),
        // A constant behind `const`; a plain property or field is not a symbol.
        ("const val LIMIT = 10", Some("LIMIT")),
        ("    private const val TAG = \"Invoice\"", Some("TAG")),
        ("        const val MAX = 1", Some("MAX")),
        ("    val all = listOf<Order>()", None),
        ("    private static final int LIMIT = 10;", None),
        ("    static final Invoice EMPTY = new Invoice(0);", None),
        // `companion object` names nothing, and a call is not a declaration.
        ("    companion object {", None),
        ("    return compute(items);", None),
        ("    Map<String, Integer> rows = compute(items);", None),
        ("    System.out.println(x);", None),
        ("    } catch (IOException e) {", None),
        ("    if (parse(x)) {", None),
        ("    Card(title = \"Invoice\") {", None),
        ("        withContext(Dispatchers.IO) {", None),
        ("        return new Runnable() {", None),
        // The constructor is listed under its class.
        ("    public Invoice(int n) {", None),
    ] {
        assert_eq!(jvm(line).as_deref(), name, "{line}");
    }
}

#[test]
fn ruby_symbol_names() {
    let rb = |line| one(Kind::Ruby, line);
    for (line, name) in [
        ("module Billing", Some("Billing")),
        ("  class Invoice < Base", Some("Invoice")),
        ("  class Billing::Invoice < Base", Some("Invoice")),
        ("    def initialize(id)", Some("initialize")),
        ("    def self.parse(text)", Some("parse")),
        ("    def Invoice.build(text)", Some("build")),
        ("    def total=(value)", Some("total")),
        ("    def empty?", Some("empty")),
        // No keyword to go by: a constant, and one `attr_accessor` line can declare
        // several names at once.
        ("  LIMIT = 10", None),
        ("    attr_accessor :total", None),
        ("    @cache ||= {}", None),
        ("    class << self", None),
        ("    Invoice.new(1)", None),
        ("  end", None),
    ] {
        assert_eq!(rb(line).as_deref(), name, "{line}");
    }
}

#[test]
fn c_symbol_names() {
    let c = |line| one(Kind::C, line);
    for (line, name) in [
        // A function: in column zero, where C has no statements, anything but a prototype.
        (
            "int invoice_total(struct invoice *inv) {",
            Some("invoice_total"),
        ),
        ("static int compute(struct invoice *inv)", Some("compute")),
        (
            "void invoice_free(struct invoice *inv,",
            Some("invoice_free"),
        ),
        (
            "sds *sdssplitlen(const char *s, ssize_t len)",
            Some("sdssplitlen"),
        ),
        ("void Ledger::append(Rows rows) {", Some("append")),
        // GNU style: the return type is on the line above, so the name starts the line.
        (
            "edata_ind_get(const edata_t *edata) {",
            Some("edata_ind_get"),
        ),
        ("static unsigned", None),
        (
            "FMT_FUNC auto vformat(string_view f) -> std::string {",
            Some("vformat"),
        ),
        ("int invoice_total(struct invoice *inv);", None),
        // A method, indented, told from a call by the body it opens.
        (
            "  auto total() const -> int { return total_; }",
            Some("total"),
        ),
        ("  explicit Ledger(int n) : total_(n) {}", Some("Ledger")),
        (
            "  template <typename T> void write(T value) {",
            Some("write"),
        ),
        ("  void append(Rows rows);", None),
        ("    if (check(rows)) {", None),
        ("    log::write(rows);", None),
        ("    return compute(inv);", None),
        ("        fmt::format_to(out, \"{}\", 42);", None),
        ("template <typename Context = context, typename... T,", None),
        // A type, a namespace and an alias.
        ("struct invoice {", Some("invoice")),
        (
            "struct __attribute__ ((__packed__)) sdshdr8 {",
            Some("sdshdr8"),
        ),
        ("static struct config {", Some("config")),
        ("template <typename T> struct Box : Base {", Some("Box")),
        (
            "struct formatter<std::filesystem::path, Char> {",
            Some("formatter"),
        ),
        ("struct client;", None),
        ("union value {", Some("value")),
        ("enum class Status {", Some("Status")),
        ("namespace billing {", Some("billing")),
        ("class LEDGER_API Ledger : public Base {", Some("Ledger")),
        (
            "class basic_memory_buffer : public detail::buffer<T> {",
            Some("basic_memory_buffer"),
        ),
        ("using Rows = std::vector<int>;", Some("Rows")),
        ("using namespace detail;", None),
        // `using a::b;` imports a name; the row would otherwise be called `a`.
        ("using std::swap;", None),
        ("  using fmt::buffered_file;", None),
        ("struct invoice *current = NULL;", None),
        // The name a typedef gives a type, once: the opening `typedef struct invoice {` is
        // not listed, so the type is one row, under the name the project writes.
        ("typedef char *sds;", Some("sds")),
        ("typedef struct redisObject robj;", Some("robj")),
        ("} invoice;", Some("invoice")),
        ("typedef struct invoice {", None),
        // Indented, a closing brace ends a nested anonymous struct: that name is a field.
        ("    } offset;", None),
        ("} while (0);", None),
        ("};", None),
        // A macro, function-like or not.
        ("#define LRU_BITS 24", Some("LRU_BITS")),
        ("#  define FMT_THROW(x) throw x", Some("FMT_THROW")),
        ("#ifndef INVOICE_H", None),
    ] {
        assert_eq!(c(line).as_deref(), name, "{line}");
    }
}

#[test]
fn csharp_symbol_names() {
    let cs = |line| one(Kind::CSharp, line);
    for (line, name) in [
        // A type, past its attributes, its modifiers and the generics it declares.
        (
            "public sealed partial class Invoice<T> : Base, IEnumerable<T>",
            Some("Invoice"),
        ),
        ("internal readonly struct Tag", Some("Tag")),
        ("public interface IStore<T> where T : class", Some("IStore")),
        ("public enum Status", Some("Status")),
        ("public record Money(decimal Amount);", Some("Money")),
        ("public record struct Point(int X, int Y);", Some("Point")),
        (
            "public delegate int Comparison<T>(T a, T b);",
            Some("Comparison"),
        ),
        // A namespace under its last part, the one `d` finds it by.
        ("namespace Billing.Core;", Some("Core")),
        ("namespace Billing", Some("Billing")),
        // A member: the type before the name is what tells it from a call.
        (
            "    public async Task<Invoice<T>> LoadAsync(int id)",
            Some("LoadAsync"),
        ),
        ("    void Save(Invoice<int> inv);", Some("Save")),
        (
            "    [Fact] public void Handles_Empty() {",
            Some("Handles_Empty"),
        ),
        (
            "    int IComparable.CompareTo(object? other) => 0;",
            Some("CompareTo"),
        ),
        (
            "    private static Rows Compute(int id) => new Rows();",
            Some("Compute"),
        ),
        // A property, by its accessors, its expression body, or the brace on the next line.
        ("    public int Total { get; private set; }", Some("Total")),
        ("    public string Name => _name;", Some("Name")),
        ("    public IReadOnlyList<int> Rows", Some("Rows")),
        // A field is not a symbol, in this kind as in every other.
        ("    private const int Limit = 10;", None),
        ("    private readonly ILogger<Invoice<T>> _logger;", None),
        ("    public static event EventHandler? Saved;", None),
        (
            "    public static Dictionary<string, Invoice<int>> All = new();",
            None,
        ),
        // The constructor is listed under its class, and a `using` alias is file-local.
        ("    public Invoice(int n)", None),
        ("using Rows = System.Collections.Generic.List<int>;", None),
        ("using System.Text.Json;", None),
        // A call, a statement and a block header are not declarations.
        ("        var rows = Compute(id);", None),
        ("        if (Check(rows))", None),
        ("        Console.WriteLine(rows);", None),
        ("        services.AddSingleton<IFoo, Foo>();", None),
        ("        return new Invoice<T>(id);", None),
        ("        foreach (var row in rows)", None),
        ("        catch (InvalidOperationException ex)", None),
        ("        using (var scope = provider.CreateScope())", None),
        ("        await client.SendAsync(request);", None),
        ("    Open,", None),
    ] {
        assert_eq!(cs(line).as_deref(), name, "{line}");
    }
}

#[test]
fn swift_symbol_names() {
    let sw = |line| one(Kind::Swift, line);
    for (line, name) in [
        ("public final class Session: NSObject {", Some("Session")),
        (
            "@MainActor public struct Response<Value> {",
            Some("Response"),
        ),
        ("actor Cache {", Some("Cache")),
        ("public enum State {", Some("State")),
        (
            "public protocol RequestDelegate: AnyObject {",
            Some("RequestDelegate"),
        ),
        ("public typealias Rows = [Int]", Some("Rows")),
        ("    associatedtype Value", Some("Value")),
        // An extension is listed under the type it extends, which is what a project's own
        // members of that type sit in.
        ("extension Session: RequestDelegate {", Some("Session")),
        ("extension Array where Element: Hashable {", Some("Array")),
        // A function, past its generics; `class func` is a static method, not a class.
        (
            "    public func request<T: Encodable>(_ url: URL) -> Request {",
            Some("request"),
        ),
        ("    class func shared() -> Session {", Some("shared")),
        ("    mutating func append(_ row: Int) {", Some("append")),
        (
            "    @discardableResult func resume() -> Self {",
            Some("resume"),
        ),
        ("    func `default`() {", Some("default")),
        // What a type holds is not a symbol, in this kind as in every other, and an `init` is
        // listed under its type.
        ("    public static let `default` = Session()", None),
        ("    private let queue: DispatchQueue", None),
        ("    public var isRunning = false", None),
        ("    case initialized", None),
        ("    public init(queue: DispatchQueue = .main) {", None),
        // An operator has no name a reader would look it up by.
        (
            "    public static func == (lhs: Self, rhs: Self) -> Bool {",
            None,
        ),
        // A call, a binding and a pattern are not declarations.
        ("        let request = Request(url)", None),
        ("        queue.async {", None),
        ("        if let delegate = delegate {", None),
        ("        guard let url = url else { return }", None),
        ("        return Session()", None),
        ("        case let .failed(error):", None),
        ("        switch request.state {", None),
    ] {
        assert_eq!(sw(line).as_deref(), name, "{line}");
    }
}

#[test]
fn php_symbol_names() {
    let php = |line| one(Kind::Php, line);
    for (line, name) in [
        (
            "abstract class Invoice implements Arrayable",
            Some("Invoice"),
        ),
        ("#[Attribute] final class Money", Some("Money")),
        ("interface Arrayable", Some("Arrayable")),
        ("trait Macroable", Some("Macroable")),
        ("enum Status: string", Some("Status")),
        ("namespace App\\Services;", Some("Services")),
        (
            "function billing_total(Invoice $invoice): int",
            Some("billing_total"),
        ),
        (
            "    final public static function parse(string $text): static",
            Some("parse"),
        ),
        (
            "    abstract protected function compute(): int;",
            Some("compute"),
        ),
        ("    public function &rows(): array", Some("rows")),
        (
            "    public const STATUS_OPEN = 'open';",
            Some("STATUS_OPEN"),
        ),
        // A property is a field, an enum case is what a type holds, and a `define()` has no
        // keyword before the name: none of them is a symbol.
        ("    protected array $rows = [];", None),
        ("    private ?Logger $logger;", None),
        ("    case Open = 'open';", None),
        ("define('BILLING_LIMIT', 10);", None),
        // A magic method is the language's hook, not the project's, as in C.
        (
            "    public function __construct(private readonly Account $account)",
            None,
        ),
        ("    public function __toString(): string", None),
        // A `use` imports, an anonymous function has no name, and a call is not a
        // declaration.
        ("use Illuminate\\Support\\Str;", None),
        ("    use Macroable;", None),
        ("$handler = function ($x) use ($y) {", None),
        ("        return $this->rows;", None),
        ("        foreach ($this->rows as $key => $value) {", None),
        ("        $total = 0;", None),
    ] {
        assert_eq!(php(line).as_deref(), name, "{line}");
    }
}

#[test]
fn the_shared_pattern_skips_the_kinds_with_rows_of_their_own() {
    // Java, Kotlin, Ruby, C, C++, Lua and Elixir are listed from their own rows only, so
    // nothing is listed twice, `def self.parse` is not `self` and `function M.setup(` is
    // not `M`.
    assert!(!shared_symbols(Some(Kind::Jvm)));
    assert!(!shared_symbols(Some(Kind::Ruby)));
    assert!(!shared_symbols(Some(Kind::C)));
    assert!(!shared_symbols(Some(Kind::CSharp)));
    assert!(!shared_symbols(Some(Kind::Swift)));
    assert!(!shared_symbols(Some(Kind::Php)));
    assert!(!shared_symbols(Some(Kind::Lua)));
    assert!(!shared_symbols(Some(Kind::Elixir)));
    // Shell and SQL rows complement the shared pattern instead, and it reads every other
    // file, known kind or not.
    assert!(shared_symbols(Some(Kind::Shell)));
    assert!(shared_symbols(Some(Kind::Sql)));
    assert!(shared_symbols(Some(Kind::Python)));
    assert!(shared_symbols(None));
}

#[test]
fn infra_symbol_names() {
    let make = |line| symbol(Some(Kind::Make), line);
    assert_eq!(make("build test: deps $(SRC)").as_deref(), Some("build"));
    assert_eq!(make("deps::").as_deref(), Some("deps"));
    assert_eq!(make("build-release:").as_deref(), Some("build-release"));
    for not_a_target in [
        ".PHONY: build",
        "%.o: %.c",
        "CC := gcc",
        "CC ::= gcc",
        "CC ?= gcc",
        "\t@echo a: b",
        "ifeq ($(OS),Windows_NT)",
        "$(OBJ): x",
    ] {
        assert_eq!(make(not_a_target), None, "{not_a_target}");
    }

    let tf = |line| symbol(Some(Kind::Terraform), line);
    assert_eq!(
        tf(r#"resource "aws_s3_bucket" "logs" {"#).as_deref(),
        Some("aws_s3_bucket.logs")
    );
    assert_eq!(
        tf(r#"data "aws_ami" "ubuntu" {"#).as_deref(),
        Some("data.aws_ami.ubuntu")
    );
    assert_eq!(tf(r#"variable "region" {"#).as_deref(), Some("var.region"));
    assert_eq!(tf(r#"module "vpc" {"#).as_deref(), Some("module.vpc"));
    assert_eq!(tf(r#"output "url" {"#).as_deref(), Some("output.url"));
    assert_eq!(tf(r#"  dynamic "ingress" {"#), None);

    let docker = |line| symbol(Some(Kind::Docker), line);
    assert_eq!(docker("FROM rust:1.80 AS build").as_deref(), Some("build"));
    assert_eq!(
        docker("from --platform=$P debian as runtime").as_deref(),
        Some("runtime")
    );
    assert_eq!(docker("FROM debian"), None);

    let yaml = |line| symbol(Some(Kind::Yaml), line);
    assert_eq!(yaml("x-common: &common").as_deref(), Some("&common"));
    assert_eq!(yaml("  <<: *common"), None);
    assert_eq!(yaml("apiVersion: v1"), None);
}

#[test]
fn word_at_covers_the_run_under_and_before_the_cursor() {
    let line = "    inv = parse_it(\"x\")";
    assert_eq!(word_at(line, 4, ""), Some((4..7, "inv")));
    assert_eq!(word_at(line, 6, ""), Some((4..7, "inv")));
    // Right after a word counts as being on it; on a space it does not.
    assert_eq!(word_at(line, 7, ""), Some((4..7, "inv")));
    assert_eq!(word_at(line, 8, ""), None);
    assert_eq!(word_at(line, 10, ""), Some((10..18, "parse_it")));
    assert_eq!(word_at(line, line.len(), ""), None);
    assert_eq!(word_at("", 0, ""), None);
    assert_eq!(word_at("x", 99, ""), Some((0..1, "x")));
}

#[test]
fn extra_word_chars_join_names_but_do_not_start_them() {
    let line = "  bucket = var.my-region[0]";
    assert_eq!(word_at(line, 17, ""), Some((15..17, "my")));
    assert_eq!(word_at(line, 17, "-"), Some((15..24, "my-region")));
    assert_eq!(word_at(line, 12, "-."), Some((11..24, "var.my-region")));
    // A flag's dashes and a trailing dot are not part of the name.
    assert_eq!(word_at("COPY --from=build", 8, "-"), Some((7..11, "from")));
    assert_eq!(word_at("x = var.", 6, "-."), Some((4..7, "var")));
    assert_eq!(word_at("- db", 0, "-"), None);
    assert_eq!(word_chars(Some(Kind::Terraform), true), "-.");
    assert_eq!(word_chars(Some(Kind::Terraform), false), "-");
    assert_eq!(word_chars(Some(Kind::Python), true), "");
    assert_eq!(word_chars(None, false), "");
}
