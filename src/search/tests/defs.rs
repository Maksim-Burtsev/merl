//! What `d`'s patterns match: the shared rules, and Python, Rust, TypeScript,
//! JavaScript, Java, Kotlin and Ruby.

use super::*;

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

/// `sys.path` can list a directory twice, far apart (a `PYTHONPATH` entry, a `.pth` file):
/// walked once, where it first stands (#152).
#[test]
#[cfg(unix)]
fn python_roots_keep_the_order_of_sys_path_and_drop_its_repeats() {
    use std::os::unix::fs::PermissionsExt;
    let (dir, _) = scratch("py-roots", &[("a/x.py", ""), ("b/x.py", "")]);
    let (a, b) = (dir.join("a"), dir.join("b"));
    // The project's interpreter, standing in for `.venv/bin/python -c 'print(sys.path)'`.
    let python = dir.join(".venv/bin/python");
    std::fs::create_dir_all(python.parent().unwrap()).unwrap();
    let script = format!(
        "#!/bin/sh\necho {0}\necho {1}\necho {0}\n",
        b.display(),
        a.display()
    );
    std::fs::write(&python, script).unwrap();
    std::fs::set_permissions(&python, std::fs::Permissions::from_mode(0o755)).unwrap();
    // A script this fresh fails to start while a fork of another test still holds it open for
    // writing (ETXTBSY); the roots are empty then, and the next try starts it.
    let roots = (0..20)
        .map(|_| external_roots(Kind::Python, &dir))
        .find(|r| !r.is_empty())
        .unwrap_or_default();
    assert_eq!(roots, [b, a]);
    std::fs::remove_dir_all(&dir).unwrap();
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
