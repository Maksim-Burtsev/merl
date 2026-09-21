//! Go: its signatures, its packages and the files a platform is built from.

use super::*;

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
