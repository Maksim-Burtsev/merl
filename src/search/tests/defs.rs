use super::*;

#[test]
fn python_def_patterns_find_declarations_only() {
    let (dir, files) = project("py");
    let py = files[..1].to_vec();
    assert_eq!(
        defs(&dir, &py, Kind::Python, "total"),
        [2],
        "the `def` line, not the `total_foobar` assignment"
    );
    assert_eq!(defs(&dir, &py, Kind::Python, "parse"), [6]);
    assert_eq!(
        defs(&dir, &py, Kind::Python, "DEFAULT_LIMIT"),
        [10],
        "a module constant, annotated or not"
    );
    assert_eq!(defs(&dir, &py, Kind::Python, "NAME_RE"), [13]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(unix)]
fn python_roots_read_the_venv_and_never_run_it() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, _) = scratch(
        "py-venv",
        &[
            ("cellar/bin/python3.99", ""),
            ("cellar/lib/python3.99/json/__init__.py", ""),
            (".venv/lib/python3.99/site-packages/lib/__init__.py", ""),
        ],
    );
    std::os::unix::fs::symlink(dir.join("cellar"), dir.join("opt")).unwrap();
    let cfg = format!(
        "home = {}\nversion = 3.99.0\n",
        dir.join("opt/bin").display()
    );
    std::fs::write(dir.join(".venv/pyvenv.cfg"), cfg).unwrap();
    let python = dir.join(".venv/bin/python");
    std::fs::create_dir_all(python.parent().unwrap()).unwrap();
    let script = format!("#!/bin/sh\ntouch {}\n", dir.join("ran").display());
    std::fs::write(&python, script).unwrap();
    std::fs::set_permissions(&python, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        external_roots(Kind::Python, &dir),
        [
            dir.canonicalize().unwrap().join("cellar/lib/python3.99"),
            dir.join(".venv/lib/python3.99/site-packages"),
        ],
        "the standard library beside the interpreter `pyvenv.cfg` names, through a symlinked \
         prefix as Homebrew's `opt` is, then the venv's `site-packages`"
    );
    assert!(
        !dir.join("ran").exists(),
        "the venv's `bin/python` is a file the repository may ship: read, never run"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn python_base_packages_are_the_venvs_only_when_it_says_so() {
    let (dir, _) = scratch(
        "py-venv-base",
        &[
            ("base/bin/python3.99", ""),
            ("base/lib/python3.99/json/__init__.py", ""),
            ("base/lib/python3.99/site-packages/pip/__init__.py", ""),
            (".venv/lib/python3.99/site-packages/lib/__init__.py", ""),
        ],
    );
    let base = dir.canonicalize().unwrap().join("base/lib/python3.99");
    let site = dir.join(".venv/lib/python3.99/site-packages");
    let home = dir.join("base/bin").display().to_string();
    let files = |include: &str| {
        let cfg = format!("home = {home}\ninclude-system-site-packages = {include}\n");
        std::fs::write(dir.join(".venv/pyvenv.cfg"), cfg).unwrap();
        let roots = external_roots(Kind::Python, &dir);
        let mut listed = external_files(Kind::Python, &roots);
        listed.sort();
        (roots, listed)
    };
    let (roots, listed) = files("false");
    assert_eq!(roots, [base.clone(), site.clone()]);
    let sorted = |mut v: Vec<PathBuf>| {
        v.sort();
        v
    };
    assert_eq!(
        listed,
        sorted(vec![
            base.join("json/__init__.py"),
            site.join("lib/__init__.py")
        ])
    );
    let (roots, listed) = files("true");
    assert_eq!(
        roots,
        [base.clone(), site.clone(), base.join("site-packages")]
    );
    assert_eq!(
        listed,
        sorted(vec![
            base.join("json/__init__.py"),
            base.join("site-packages/pip/__init__.py"),
            site.join("lib/__init__.py"),
        ])
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_root_inside_another_is_walked_once() {
    let (dir, _) = scratch(
        "nested-roots",
        &[("json/__init__.py", ""), ("vendor/foo/__init__.py", "")],
    );
    let roots = [dir.clone(), dir.join("vendor")];
    assert_eq!(
        external_files(Kind::Python, &roots),
        [
            dir.join("json/__init__.py"),
            dir.join("vendor/foo/__init__.py")
        ]
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(unix)]
fn toolchains_are_not_picked_by_the_project() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, _) = scratch("toolchains", &[("go.mod", "module x\n\ngo 1.99.0\n")]);
    let rustc = dir.join("tc/bin/rustc");
    std::fs::create_dir_all(rustc.parent().unwrap()).unwrap();
    let script = format!("#!/bin/sh\ntouch {}\n", dir.join("ran").display());
    std::fs::write(&rustc, script).unwrap();
    std::fs::set_permissions(&rustc, std::fs::Permissions::from_mode(0o755)).unwrap();
    let toml = format!("[toolchain]\npath = \"{}\"\n", dir.join("tc").display());
    std::fs::write(dir.join("rust-toolchain.toml"), toml).unwrap();
    external_roots(Kind::Rust, &dir);
    assert!(
        !dir.join("ran").exists(),
        "a `rust-toolchain.toml` whose `path` is a `rustc` of its own is not run under rustup"
    );
    let installed = external_roots(Kind::Go, Path::new("/"));
    if installed.is_empty() {
        eprintln!("no go, skipped");
    } else {
        assert_eq!(
            external_roots(Kind::Go, &dir).first(),
            installed.first(),
            "a `go.mod` asking for a Go that does not exist neither sends Go to download it \
             nor leaves `d` without the standard library"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

const PY_MEMBERS: &str = "import asyncio\n\n\nclass UserRepository:\n    async def delete_user(self, user_id: int) -> None:\n        pass\n\n    def find_user(self, user_id):\n        return user_id\n\n\ndef find_user(user_id):\n    return user_id\n\n\nasync def main():\n    pass\n\n\ndelete_user = None\n";

#[test]
fn python_members_are_indented_defs_sync_or_async() {
    let (dir, files) = scratch("py-members", &[("repos.py", PY_MEMBERS)]);
    assert_eq!(
        defs(&dir, &files, Kind::Python, "delete_user"),
        [5, 20],
        "`async def` in a class declares for a bare word too"
    );
    assert_eq!(
        defs(&dir, &files, Kind::Python, "main"),
        [16],
        "so does a top-level `async def`"
    );
    assert_eq!(
        members(&dir, &files, Kind::Python, "delete_user"),
        [5],
        "`x.delete_user` reaches the method, not the module-level name of the same spelling"
    );
    assert_eq!(
        members(&dir, &files, Kind::Python, "find_user"),
        [8],
        "`x.find_user` reaches the method; the module-level function takes an import"
    );
    assert_eq!(defs(&dir, &files, Kind::Python, "find_user"), [8, 12]);
    std::fs::remove_dir_all(&dir).unwrap();
}

const TS_MEMBERS: &str = "export interface Repo {\n  deleteUser(id: string): Promise<void>;\n  findUser?<T>(id: string): T | undefined\n  onChange: (id: string) => void;\n  name: string;\n}\n\nexport abstract class Base {\n  abstract deleteUser(id: string): Promise<void>;\n  get size(): number;\n}\n\nconst deleteUser = (id: string) => id;\nfindUser(id);\nrun(x).then((y): void => y);\nconst n = cond ? findUser(a) : b;\nexport const helpers = {\n  deleteUser(id) {\n    return id;\n  },\n};\nappend(target, visitor ? visitNode(s) : s);\nlog(\"(while reading XRef): \" + e);\ndeclare class Emitter {\n  on(event: string, cb: (x: T) => void): this;\n  append(...items: string[]): void;\n}\nclass Session {\n  close(): void {}\n}\nnoop(() => {})\nclass Svc {\n  constructor(\n    @Inject(W) private worker: Worker,\n  ) {}\n}\nclass One {\n  constructor(private readonly inline: Dep, public other: Dep) {}\n}\ndeclare class Wide {\n  pong<T extends Record<string, number>>(x: T): T;\n}\n";

#[test]
fn ts_members_include_signatures_without_a_body() {
    let (dir, files) = scratch("ts-members", &[("repo.ts", TS_MEMBERS)]);
    let m = |w| members(&dir, &files, Kind::TsJs, w);
    assert_eq!(
        m("deleteUser"),
        [2, 9, 18],
        "the interface signature, the abstract one and the object-literal method; not the \
         `const` arrow function, a local to whoever reads `deleteUser` bare"
    );
    assert_eq!(
        defs(&dir, &files, Kind::TsJs, "deleteUser"),
        [2, 9, 13, 18],
        "a bare word reads the `const` too"
    );
    assert_eq!(
        m("findUser"),
        [3],
        "an optional generic signature with no `;`, not the call statement or the ternary"
    );
    assert_eq!(m("onChange"), [4], "a property holding a function");
    assert_eq!(m("size"), [10], "a getter signature");
    assert_eq!(m("name"), Vec::<usize>::new(), "a plain field has no rule");
    assert_eq!(
        m("run"),
        Vec::<usize>::new(),
        "an annotated arrow argument is not a signature"
    );
    assert_eq!(
        m("append"),
        [26],
        "a signature whose parameter is a function type, not the call on line 22"
    );
    assert_eq!(
        m("log"),
        Vec::<usize>::new(),
        "a call whose arguments hold `) :` or `):` is no signature"
    );
    assert_eq!(m("on"), [25], "a signature with a rest parameter");
    assert_eq!(m("close"), [29], "an empty body on the method's line");
    assert_eq!(
        m("noop"),
        Vec::<usize>::new(),
        "a call whose last argument is an empty body is no method"
    );
    assert_eq!(
        m("worker"),
        [34],
        "a constructor parameter behind an access modifier is a property of the class"
    );
    assert_eq!(m("inline"), [38]);
    assert_eq!(m("other"), [38]);
    assert_eq!(m("pong"), [41], "type parameters may nest");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_literal_ends_where_the_language_ends_it() {
    assert_eq!(
        uncommented(Kind::Python, "sep = \"\\\\\"  # note"),
        "sep = \"\\\\\"  ",
        "`\"\\\\\"` ends on its second quote"
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
        "fn f<'a>(x: &str) ",
        "a Rust lifetime opens no literal"
    );
    assert_eq!(
        uncommented(Kind::C, "int n = 1'000; // note"),
        "int n = 1'000; ",
        "nor does a C++ digit separator"
    );
    assert_eq!(
        uncommented(Kind::Rust, "let q = '\"'; // note"),
        "let q = '\"'; ",
        "a char literal still hides what it holds"
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
    let ts = "const q = `\\`\nfunction ghost() {}\n`;\nfunction real() {}\n";
    assert_eq!(
        inside(Kind::TsJs, ts),
        [2, 3],
        "a template writes a backtick as `\\``"
    );
    assert!(
        inside(Kind::Go, "const q = `\\`\nfunc real() {}\n").is_empty(),
        "a Go raw string has no escapes: it ends at the `\\``"
    );
    let ts = "const a = `${b ? `${c}/` : \"\"}${d}`;\nfunction real() {}\nconst e = `\n${`\nnested`}\n`;\nclass After {}\n";
    assert_eq!(
        inside(Kind::TsJs, ts),
        [4, 5, 6],
        "a template inside a template's `${{…}}` closes there, and so does the `${{…}}`"
    );
    let ts = "const a = `${s.replace(/\\{/g, \"\")}`;\nfunction real() {}\nconst b = `${t.split(/\\}/)}`;\nclass After {}\n";
    assert!(
        inside(Kind::TsJs, ts).is_empty(),
        "a regex's `\\{{` or `\\}}` in a `${{…}}` is no brace of it"
    );
    let cs = "var q = \"\"\"\n    WHERE EXISTS(SELECT 1 FROM t)\n    \"\"\";\nvar v = @\"\n    SELECT MIN(\"\"rowid\"\") FROM t\n    \";\npublic int Real() => 1;\n";
    assert_eq!(
        inside(Kind::CSharp, cs),
        [2, 3, 5, 6],
        "the SQL of a raw and a verbatim string, where `\"\"` writes a quote and closes nothing"
    );
    let cs = "var a = @\"C:\\\";\nvar b = $@\"{d}\\\";\nvar c = @$\"\n    {d}\\\";\nvar e = \"a\\\" /* b\";\nvoid Real() {}\n";
    assert_eq!(
        inside(Kind::CSharp, cs),
        [4],
        "a backslash escapes nothing in a verbatim string, `$@\"` and `@$\"` included, and \
         escapes a quote in a regular one: the `/*` after `\\\"` is inside the string"
    );
    let sw = "let doc = \"\"\"\n    class Ghost {}\n    \"\"\"\nclass Real {}\n";
    assert_eq!(inside(Kind::Swift, sw), [2, 3]);
    let php = "$sql = <<<SQL\n    function ghost() {}\n    class Ghost {}\nSQL;\n$n = <<<'TXT'\n    class Nowdoc {}\nTXT;\nclass Real {}\n";
    assert_eq!(
        inside(Kind::Php, php),
        [2, 3, 6],
        "a heredoc ends on the line that repeats its label, and only there"
    );
    let php =
        "<?php\n# loads lib/*\nfunction below() {}\n#[Route('/api')] /*\nfunction ghost() {}\n*/\n";
    assert_eq!(
        inside(Kind::Php, php),
        [5, 6],
        "`#` is a line comment as `//` is, but `#[` opens an attribute, whose `/*` does"
    );
    let php = "<?php # render ?><script>const t = `\nfunction ghost() {}\n`;</script>\n<style>#a { color: red; } /*\nfunction ghost() {}\n*/</style>\n<script>class W {\n  #tpl = `\n<b></b>\n`;\n}</script>\n<?php\nfunction real() {}\n";
    assert_eq!(
        inside(Kind::Php, php),
        [2, 3, 5, 6, 9, 10],
        "outside `<?php … ?>` a `#` is HTML's, CSS's or JS's, and a comment ends at `?>`"
    );
}

fn inside(kind: Kind, text: &str) -> Vec<usize> {
    let lines = literal_lines(kind, text);
    (1..=lines.len()).filter(|&n| lines[n - 1]).collect()
}

#[test]
fn a_rust_string_runs_over_lines() {
    let rs = "const A: &str = \"\\\\\";\nfn real() {}\nconst U: &str = \"a \\\" /* \\\nfn ghost() {}\n\";\nfn f<'a>(x: &'a str) -> &'a str { let r#type = x; r#type }\nconst R: &str = r#\"say \"hi\"\nfn ghost() {}\n\"#;\nlet c = '\"'; let g = \"**/*.rs\"; let e = '\\u{1F600}'; let b = br##\"#\"##;\nfn real2() {}\n/// [`Foo`] \"doc\nfn documented() {}\n";
    assert_eq!(inside(Kind::Rust, rs), [4, 5, 8, 9]);
    let at = |s: &str| rs.find(s).unwrap();
    for (word, is) in [
        ("ghost", true),
        ("*/*.rs", true),
        ("hi\"", true),
        ("real2", false),
        ("a str", false),
        ("Foo", false),
        ("documented", false),
    ] {
        assert_eq!(in_string(Kind::Rust, rs, at(word)), is, "{word}");
    }
    assert!(!in_string(Kind::TsJs, "const s = \"ghost\";", 12));
}

#[test]
fn ruby_literals_are_read_as_ruby_writes_them() {
    let rb = "# don't `touch\ndef a; end\nx = <<~SQL + <<-'B' # two\n  def ghost1\nSQL\n  def ghost2\n  B\ndef b; end\nlist << item\nclass << self\ndef c; end\ny = <<X\n  X\ndef ghost3\nX\np = /#/ && <<~Q\ndef ghost4\nQ\n=begin\ndef ghost5\n=end\ndef d; end\n`echo #{1} '`\ndef e; end\n__END__\ndef ghost6\n";
    assert_eq!(
        inside(Kind::Ruby, rb),
        [4, 6, 13, 14, 17, 20, 26, 27],
        "two heredocs on a line in order, a plain `<<X` closed only by `X` at the margin, \
         `=begin` and `__END__`; not a `<<` that opens no heredoc, nor a regex's `#`"
    );
    assert!(
        inside(Kind::Ruby, "# `\n/* x\ndef a; end\n").is_empty(),
        "no backtick template, no `/* */` block"
    );
}

#[test]
fn a_cpp_raw_string_runs_to_its_delimiter() {
    let cc = "auto a = R\"(\nstruct Ghost {\n)\";\nauto b = u8R\"x(a )\" b\nstruct Ghost2 {\n)x\";\nauto c = LR\"(x)\"; int real;\nauto d = FOOR\"(\";\nstruct Real {};\n";
    assert_eq!(
        inside(Kind::C, cc),
        [2, 3, 5, 6],
        "with a delimiter and behind an encoding prefix; an `R` that ends a longer name opens \
         nothing"
    );
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
    assert_eq!(
        defs(&dir, &go, Kind::Go, "inv"),
        [12],
        "`inv := Parse(\"x\")` is the closest thing Go has to a definition of `inv`"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn rust_def_patterns_cover_items_behind_prefixes() {
    let (dir, files) = project("rs");
    let rs = files[2..3].to_vec();
    assert_eq!(
        defs(&dir, &rs, Kind::Rust, "Order"),
        [1],
        "the struct, not the `impl` block or the `Order {{ .. }}` literal"
    );
    for (word, line) in [
        ("sum", 6),
        ("MAX_ORDERS", 9),
        ("parse_order", 11),
        ("orders", 21),
    ] {
        assert_eq!(defs(&dir, &rs, Kind::Rust, word), [line], "{word}");
    }
    assert_eq!(
        defs(&dir, &rs, Kind::Rust, "order"),
        [17],
        "the macro, not the `let mut` binding: a local is its block's, never found by name"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn ts_def_patterns_cover_declarations_methods_and_arrows() {
    let (dir, files) = project("ts");
    let ts_js = files[3..].to_vec();
    for (word, file, line) in [
        ("Order", "d.ts", 1),
        ("Id", "d.ts", 5),
        ("Status", "d.ts", 7),
        ("OrderService", "d.ts", 11),
        ("load", "d.ts", 14),
        ("sum", "d.ts", 19),
        ("parseOrder", "d.ts", 22),
        ("render", "d.ts", 24),
        ("n", "d.ts", 25),
        ("ids", "d.ts", 28),
        ("parse", "e.js", 2),
        ("format", "e.js", 5),
    ] {
        let pat = def_patterns(Kind::TsJs, word).join("|");
        assert_eq!(
            lines(&grep(&dir, &ts_js, &pat, false, false)),
            [(file.into(), line)],
            "{word}: one kind for `.ts` and `.js`, and the declaration, never a call such as \
             `render(o);`"
        );
    }
    assert!(
        defs(&dir, &ts_js, Kind::TsJs, "cache").is_empty(),
        "a plain field is not a declaration the rules know"
    );
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
    assert_eq!(
        d("Invoice"),
        [3, 7],
        "the class and the constructor; the caller shows a picker"
    );
    assert_eq!(d("LIMIT"), [4]);
    assert_eq!(d("items"), [5], "not the `compute(items)` call");
    assert_eq!(d("total"), [12], "behind its annotation and modifiers");
    assert_eq!(
        d("compute"),
        [16],
        "not the `return compute(items);` call above it"
    );
    assert_eq!(d("onSave"), [20], "no modifier, but a return type");
    assert_eq!(d("run"), [22]);
    assert_eq!(d("Store"), [27]);
    assert_eq!(d("save"), [28], "an interface method has no body");
    assert_eq!(d("Point"), [31]);
    assert_eq!(d("Status"), [33]);
    assert_eq!(d("rows"), Vec::<usize>::new(), "a parameter has no rule");
    assert_eq!(
        d("Runnable"),
        Vec::<usize>::new(),
        "`new Runnable() {{` opens an anonymous class: a use of the interface"
    );
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
    assert_eq!(
        d("Card"),
        [17],
        "the `fun`, not the two `Card(title = …) {{` calls with a trailing lambda"
    );
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

#[test]
fn scala_declares_and_refuses() {
    let declares = |word: &str, line: &str| {
        Regex::new(&def_patterns(Kind::Jvm, word).join("|"))
            .unwrap()
            .is_match(line)
    };
    for (word, line) in [
        ("Invoice", "class Invoice(val id: Long) {"),
        ("User", "case class User(name: String)"),
        ("Shape", "sealed abstract class Shape"),
        ("RichInt", "implicit class RichInt(x: Int)"),
        ("Repo", "trait Repo[F[_]]:"),
        ("Ledger", "object Ledger:"),
        ("Empty", "case object Empty"),
        ("shop", "package object shop {"),
        ("Color", "enum Color:"),
        ("total", "  def total(xs: List[Int]): Int ="),
        ("toString", "  override def toString = s\"x\""),
        ("load", "  private[shop] def load()"),
        ("load", "  protected[this] def load()"),
        ("debug", "  inline def debug(msg: String): Unit = ()"),
        ("slug", "extension (s: String) def slug: String ="),
        ("limit", "  val limit = 10"),
        ("count", "  var count = 0"),
        ("core", "lazy val core = project"),
        ("ec", "  implicit val ec: ExecutionContext = global"),
        ("Id", "  type Id = Long"),
        ("UserId", "opaque type UserId = Long"),
        ("T", "  type T <: Animal"),
        (
            "userOrdering",
            "given userOrdering: Ordering[User] = Ordering.by(_.name)",
        ),
        ("userOrdering", "given userOrdering: Ordering[User] with"),
        ("Green", "  case Red, Green, Blue"),
        ("Circle", "  case Circle(r: Double)"),
        ("Mercury", "  case Mercury extends Planet(3.3e23)"),
        ("id", "class Invoice(val id: Long, var paid: Boolean)"),
        ("paid", "class Invoice(val id: Long, var paid: Boolean)"),
        ("age", "case class User(name: String, age: Int)"),
        ("type", "  val `type` = 1"),
        ("x", "implicit class RichInt(val x: Int)"),
        ("x", "class Box(private[shop] val x: Int)"),
    ] {
        assert!(declares(word, line), "{word} is declared by {line}");
    }
    for (word, line) in [
        ("Invoice", "  val x = new Invoice(1)"),
        ("Invoice", "  val x = Invoice(1)"),
        ("total", "  Ledger.total(xs)"),
        ("Invoice", "import shop.{Invoice, Order => O}"),
        ("Invoice", "  def f(x: Invoice): Unit"),
        ("Invoice", "class Paid extends Invoice(1)"),
        ("Logging", "class Paid extends Base with Logging"),
        ("total", "  export Ledger.total"),
        ("Invoice", "    case Invoice(id, _) =>"),
        ("Red", "    case Red | Green =>"),
        ("a", "  val (a, b) = pair"),
        ("RED", "            case RED:"),
        ("GREEN", "            case RED, GREEN:"),
        ("RED", "            case RED -> 1;"),
        ("RED", "            case RED: return 1;"),
        ("Ordering", "given Ordering[User] = Ordering.by(_.name)"),
        ("xs", "  def total(xs: List[Int]): Int ="),
        ("name", "  name: String,"),
        ("name", "class Invoice(name: String)"),
        ("plus", "  def +(other: Money): Money = this"),
        ("this", "  def this(x: Int) = this(x, 0)"),
    ] {
        assert!(!declares(word, line), "{word} is not declared by {line}");
    }
}

#[test]
fn scala_scopes_and_packages() {
    let kt = "fun f(trait: List<Int>) {\n    trait.let {\n        val count = it.size\n    }\n}\n";
    assert!(
        jvm_local_block(kt, 3).is_some(),
        "a Kotlin variable called `trait` opens no type: a lambda's local stays a local"
    );
    let obj = "object Config {\n    val limit = 3\n}\n";
    assert_eq!(
        jvm_local_block(obj, 2),
        None,
        "a named Kotlin `object` opens a type: its member is no local"
    );
    assert_eq!(
        jvm_package("package object shop {\n  val x = 1\n}\n"),
        None,
        "`package object` is no package"
    );

    assert_eq!(
        jvm_package("package ledger\n\npackage object teller {\n"),
        Some("ledger".into())
    );
    assert_eq!(jvm_package("package a.`fun`.b\n"), Some("a".into()));
    assert!(scala_type_parameter("  def f[A: Ordering](a: A) = a", "A"));
    assert!(scala_type_parameter("trait Repo[F[_]]:", "F"));
    assert!(!scala_type_parameter("  val xs = arr[Item]", "Item"));
    let ext = "extension (s: String)\n  def shout: String = s\n";
    assert_eq!(jvm_receiver_at(ext, 2, "shout"), Some("String".into()));
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
    assert_eq!(d("total"), [5], "the accessor, not `total == other.total`");
    assert_eq!(
        d("@total"),
        [20],
        "the `@total` the setter sets is a word of its own"
    );
    assert_eq!(d("total="), [5, 19], "the setter itself is `total=`");
    assert_eq!(d("customer"), [6], "second in the `attr_reader` list");
    assert_eq!(d("id"), [6], "`id => 1,` is a hash pair, not an assignment");
    assert_eq!(d("@id"), [11]);
    assert_eq!(d("@@count"), [8], "a class variable");
    assert_eq!(d("count"), Vec::<usize>::new());
    assert_eq!(d("@count"), Vec::<usize>::new());
    assert_eq!(d("initialize"), [10]);
    assert_eq!(d("parse"), [15], "`def self.parse`");
    assert_eq!(d("cache"), [23]);
    assert_eq!(d("@cache"), [24], "the `||=` it memoises with");
    assert_eq!(d("empty?"), [27], "`?` is part of the name");
    assert_eq!(d("empty"), Vec::<usize>::new(), "`@rows.empty?` is a call");
    assert_eq!(d("rows"), Vec::<usize>::new());
    assert_eq!(d("@rows"), [12]);
    assert_eq!(d("blank?"), [41]);
    assert_eq!(d("blank"), Vec::<usize>::new());
    assert_eq!(d("name"), Vec::<usize>::new(), "`name =~ /x/` is a match");
    assert_eq!(d("new"), Vec::<usize>::new());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]

fn an_iife_or_a_umd_factory_declares_at_the_top_of_its_script() {
    let nested = |text: &str, line| ts_nested_local(&text.lines().collect::<Vec<_>>(), line);
    for head in [
        "(function () {",
        ";(function (window) {",
        "!function () {",
        "(() => {",
        "(async () => {",
        "}(this, (function (exports) { 'use strict';",
        "})(self, () => {",
    ] {
        let js = format!("{head}\n  function top() {{\n    const inner = 1;\n  }}\n");
        assert!(!nested(&js, 2), "{head}");
        assert!(nested(&js, 3), "{head}");
    }
    assert!(
        nested("function f() {\n  function g() {}\n}\n", 2),
        "a function inside one of its functions stays a local"
    );
}

fn declares_line(kind: Kind, word: &str, line: &str) -> bool {
    Regex::new(&def_patterns(kind, word).join("|"))
        .unwrap()
        .is_match(line)
}

#[test]
fn a_rust_let_is_never_found_by_name() {
    for line in ["    let order = 1;", "    let mut order = Order::new();"] {
        assert!(!declares_line(Kind::Rust, "order", line), "{line}");
    }
    assert!(declares_line(Kind::Rust, "order", "fn order() {}"));
}

#[test]
fn a_kotlin_extension_property_declares_its_name_not_its_receiver() {
    let line = "val Topic.testTag: String get() = \"t\"";
    assert!(declares_line(Kind::Jvm, "testTag", line));
    assert!(!declares_line(Kind::Jvm, "Topic", line));
}

#[test]
fn a_java_record_declares_its_components_on_a_one_line_header() {
    for (word, line) in [
        ("x", "record Point(int x, int y) {}"),
        ("y", "record Point(int x, int y) {}"),
        ("hi", "public record Range(@NotNull Integer lo, long hi) {"),
    ] {
        assert!(declares_line(Kind::Jvm, word, line), "{word}: {line}");
    }
}

#[test]
fn csharp_declarations_are_told_from_uses() {
    let cs = |word, line| declares_line(Kind::CSharp, word, line);
    assert!(cs(
        "Rows",
        "using Rows = System.Collections.Generic.List<int>;"
    ));
    assert!(cs("Invoice", "    public Invoice(int n)"));
    for (word, line) in [
        ("Json", "using System.Text.Json;"),
        ("Text", "using System.Text.Json;"),
        ("Invoice", "            new Invoice(1)"),
        ("Invoice", "            new Invoice(1) {"),
        ("Register", "        Register("),
        ("Configure", "        Configure(options =>"),
    ] {
        assert!(!cs(word, line), "{line}");
    }
}

#[test]
fn an_elixir_defimpl_declares_no_module_of_its_own() {
    assert!(!declares_line(
        Kind::Elixir,
        "Encoder",
        "  defimpl Jason.Encoder do"
    ));
    assert!(declares_line(
        Kind::Elixir,
        "Ledger",
        "defmodule MyApp.Ledger do"
    ));
}

#[test]
fn a_zig_test_is_listed_but_never_declares_a_word() {
    let line = "test \"parse reads a header\" {";
    assert_eq!(
        one(Kind::Zig, line).as_deref(),
        Some("parse reads a header")
    );
    assert!(!declares_line(Kind::Zig, "parse", line));
}

#[test]
fn a_proto_rpc_is_declared_braces_or_not_with_stream_arguments() {
    for line in [
        "  rpc Watch(stream WatchRequest) returns (stream Event);",
        "  rpc Watch(WatchRequest) returns (Event) {}",
    ] {
        assert!(declares_line(Kind::Proto, "Watch", line), "{line}");
    }
}

#[test]
fn a_rebind_a_write_or_a_column_declares_nothing() {
    for (kind, word, line) in [
        (
            Kind::Swift,
            "delegate",
            "        if let delegate = delegate {",
        ),
        (
            Kind::Swift,
            "url",
            "        guard let url = url else { return }",
        ),
        (Kind::Php, "rows", "        $this->rows = [];"),
        (Kind::Php, "row", "        foreach ($rows as $row) {"),
        (Kind::Sql, "total", "  total numeric NOT NULL,"),
    ] {
        assert!(!declares_line(kind, word, line), "{line}");
    }
    assert!(declares_line(Kind::Php, "rows", "        $rows = [];"));
}

#[test]
fn proto_names_take_a_negative_number_a_qualified_type_and_no_extend() {
    let proto = |word, line| declares_line(Kind::Proto, word, line);
    assert!(proto("CHANNEL_UNKNOWN", "  CHANNEL_UNKNOWN = -1;"));
    assert!(proto("owner", "  .shop.v1.User owner = 4;"));
    assert!(!proto("Tariff", "extend Tariff {"));
}

#[test]
fn a_graphql_field_reads_its_type_past_a_comment_in_column_zero() {
    let lines = ["type User {", "# who wrote it", "  email: String!", "}"];
    assert!(graphql_member(&lines, 3));
}

fn c_hit(path: &str, line1: usize, text: &str) -> Hit {
    Hit {
        path: PathBuf::from(path),
        line1,
        byte_col: None,
        text: text.into(),
        deleted: None,
    }
}

#[test]
fn a_file_scope_static_of_the_file_on_screen_hides_the_rest_unless_the_cursor_is_on_one() {
    let here = Path::new("a.c");
    let here_text = "static int count;\n";
    let hits = || {
        vec![
            c_hit("a.c", 1, "static int count;"),
            c_hit("b.h", 3, "extern int count;"),
        ]
    };
    let rows = |on| {
        c_file_local("count", here, here_text, hits(), |_| None, on)
            .into_iter()
            .map(|h| h.path)
            .collect::<Vec<_>>()
    };
    assert_eq!(rows(false), [PathBuf::from("a.c")]);
    assert_eq!(rows(true), [PathBuf::from("a.c"), PathBuf::from("b.h")]);
}

#[test]
fn a_type_in_another_source_files_unnamed_namespace_stays_a_candidate() {
    let other = "namespace {\nstruct Impl {\n};\n}\n";
    let rows = c_file_local(
        "Impl",
        Path::new("a.cc"),
        "",
        vec![c_hit("b.cc", 2, "struct Impl {")],
        |p| (p == Path::new("b.cc")).then(|| other.to_string()),
        false,
    );
    assert_eq!(rows.len(), 1);
}

#[test]
fn a_static_of_another_source_file_is_seen_where_that_file_is_included() {
    let rows = |here_text| {
        c_file_local(
            "count",
            Path::new("a.c"),
            here_text,
            vec![c_hit("b.c", 1, "static int count;")],
            |_| Some(String::new()),
            false,
        )
        .len()
    };
    assert_eq!(rows("#include \"b.c\"\n"), 1);
    assert_eq!(rows(""), 0);
}
#[test]
fn every_go_rule_builds_a_small_automaton_with_ascii_classes() {
    for pat in def_patterns(Kind::Go, "Total") {
        assert!(
            regex::RegexBuilder::new(&pat)
                .size_limit(16 * 1024)
                .build()
                .is_ok(),
            "{pat}"
        );
    }
}

#[test]
fn an_objective_c_plus_plus_file_is_compiled_alone() {
    let rows = c_file_local(
        "count",
        Path::new("a.c"),
        "",
        vec![c_hit("b.mm", 1, "static int count;")],
        |_| Some(String::new()),
        false,
    );
    assert!(rows.is_empty());
}

#[test]
fn a_prototype_and_a_definition_naming_a_built_in_type_are_one_function() {
    let text = "int f(unsigned long);\nint f(unsigned long n) {\n  return 0;\n}\n";
    let hits = [
        c_hit("a.c", 1, "int f(unsigned long);"),
        c_hit("a.c", 2, "int f(unsigned long n) {"),
    ];
    let one = c_one_definition("f", &hits, |_| Some(text.to_string()));
    assert_eq!(one.map(|o| o.hit_index), Some(1));
}

#[test]
fn a_prototype_puts_no_parameter_in_scope_below_it() {
    assert!(c_parameter("int f(int n)\n{\n  return n;\n}\n", 3, "n"));
    assert!(!c_parameter("int g(int n);\n  n = 1;\n", 2, "n"));
}

#[test]
fn a_typescript_signature_has_no_equals_sign_in_its_return_type() {
    let members = member_patterns(Kind::TsJs, "next").unwrap().join("|");
    let re = Regex::new(&members).unwrap();
    assert!(re.is_match("  next(value: number): number;"));
    assert!(!re.is_match("  next(value: number): number => value;"));
}
