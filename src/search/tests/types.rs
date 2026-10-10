use super::*;

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
    assert_eq!(
        bases(Kind::TsJs, TS_IMPLS, 14),
        ["Base"],
        "a wrapped header carries each clause on a line of its own"
    );
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
    assert_eq!(
        returns(Kind::Python, py, 9),
        Some(Value::New("Repo".into())),
        "no annotation: what every `return` constructs"
    );
    assert_eq!(returns(Kind::Python, py, 12), Some(ty("Repo")));
    assert_eq!(
        returns(Kind::Python, py, 14),
        Some(ty("Annotated[Repo, \"doc: x\"]")),
        "a `: ` inside a string is not the end of the annotation"
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
    assert_eq!(
        returns(Kind::TsJs, ts, 8),
        None,
        "a property holding a function is not read"
    );
    assert_eq!(returns(Kind::TsJs, ts, 11), Some(ty("Repo")));
    assert_eq!(returns(Kind::TsJs, ts, 12), Some(ty("Repo")));
    let go = "func (d *Depot) Repo(id int) *Repo {\n\treturn d.people\n}\nfunc (d Depot) Trail() (Trail, error) {\n\treturn Trail{}, nil\n}\ntype Source interface {\n\tSource() *Repo\n\tClose()\n}\n";
    assert_eq!(returns(Kind::Go, go, 1), Some(ty("*Repo")));
    assert_eq!(returns(Kind::Go, go, 4), Some(ty("Trail")));
    assert_eq!(returns(Kind::Go, go, 8), Some(ty("*Repo")));
    assert_eq!(returns(Kind::Go, go, 9), None);
}

#[test]
fn a_backslash_at_the_end_of_a_line_keeps_the_line_count() {
    let py = "log(\"a \\\n    b\")\nledger = A()\nledger.go()";
    assert_eq!(
        literal_lines(Kind::Python, py),
        [false; 4],
        "a string continued with a backslash swallows no end of a line"
    );
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
    for continues in [false, true] {
        let got = python_statements("flag): x = 1; y = 2", continues);
        assert_eq!(
            got,
            ["x = 1", "y = 2"],
            "the last line of a wrapped header, whatever the line above it ends in"
        );
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
    let pycast = |callee: &str, t: &str| Value::Cast {
        callee: callee.into(),
        written_type: t.into(),
    };
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
    assert_eq!(
        v(Kind::TsJs, "ok ? a : b as Repo"),
        Value::Unknown,
        "`as` takes the operand next to it, not the whole expression"
    );
    assert_eq!(v(Kind::TsJs, "a ?? b as Repo"), Value::Unknown);
    assert_eq!(v(Kind::TsJs, "() => row as Repo"), Value::Unknown);
    assert_eq!(
        v(Kind::TsJs, "check(x as Foo) ? other : found as Repo"),
        Value::Unknown
    );
    assert_eq!(v(Kind::TsJs, "new Wrapper(x as Foo) as Repo"), ty("Repo"));
    assert_eq!(
        v(Kind::TsJs, "load(row as Repo)"),
        Value::Call("load".into()),
        "an `as` inside brackets or a string casts something else"
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
    assert_eq!(
        at(14),
        [(10, ty("*Audit"))],
        "past a plain `switch` inside the arm, and not the closed type switch above"
    );
    assert_eq!(
        at(19),
        [],
        "no type switch: nothing the rules read binds `v` here"
    );
    let chans = "func f(v *Repo, x any, ch chan int) {\n\tswitch v := x.(type) {\n\tcase *Audit:\n\t\tv.Go()\n\t}\n\tselect {\n\tcase <-ch:\n\t\tv.Go()\n\t}\n}\n";
    assert_eq!(
        bound_at(Kind::Go, chans, 8, "v"),
        [(1, ty("*Repo"))],
        "a `select` ends its `case` as a `switch` does: the closed type switch above it is not \
         read with the `case` of the `select`, and the parameter `v` stays what it is"
    );
}

#[test]
fn a_call_wrapped_onto_the_next_lines_reads_no_arguments() {
    let v = |kind, e| value_of(kind, e);
    assert_eq!(v(Kind::Python, "cast("), Value::Unknown);
    assert_eq!(v(Kind::Python, "cast(Repo,"), Value::Unknown);
    assert_eq!(v(Kind::Go, "new("), Value::Unknown);
    assert_eq!(v(Kind::Go, "make("), Value::Unknown);
    assert_eq!(v(Kind::Go, "make([]*Repo,"), Value::Unknown);
    assert_eq!(
        v(Kind::Python, "load("),
        Value::Call("load".into()),
        "a call that is not read for a type still names its callee, wrapped or not"
    );
    assert_eq!(
        v(Kind::Python, "cast(Repo, row)"),
        Value::Cast {
            callee: "cast".into(),
            written_type: "Repo".into(),
        },
        "the same calls closed on their line keep writing the type they always did"
    );
    assert_eq!(v(Kind::Go, "new(Repo)"), Value::New("Repo".into()));
    assert_eq!(v(Kind::Go, "make([]*Repo, 0, 10)"), ty("[]*Repo"));
    let py = "def handler(container):\n    repo = cast(\n        Repo, container.get(\"repo\")\n    )\n    repo.save()\n";
    assert_eq!(
        bindings(Kind::Python, py, 5, "repo"),
        [Binding {
            line1: 2,
            value: Value::Unknown
        }],
        "`d` on a name bound by a wrapped call answers"
    );
    let go = "func f() {\n\trepos := make(\n\t\t[]*Repo, 0, 10,\n\t)\n\trepos[0].Save()\n}\n";
    assert_eq!(
        bindings(Kind::Go, go, 5, "repos"),
        [Binding {
            line1: 2,
            value: Value::Unknown
        }]
    );
}

#[test]
fn a_chain_may_hang_off_the_call_that_starts_it() {
    let head = |kind, line: &str| {
        let at = line.rfind('.').unwrap() + 1;
        call_head(kind, line, at).map(|h| {
            (
                h.call_without_arguments,
                h.value,
                h.fields_to_word.join("."),
            )
        })
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
            Value::Cast {
                callee: "cast".into(),
                written_type: "Repo".into(),
            },
            String::new()
        ))
    );
    assert_eq!(
        head(Kind::TsJs, "  (await load()).find()"),
        call("load()", "load", ""),
        "brackets around a call are read through, as an `await` in front of one is"
    );
    assert_eq!(
        head(Kind::Go, "\tOpen().Repo().Delete()"),
        None,
        "a call of a call"
    );
    assert_eq!(
        head(Kind::Python, "    make()[0].delete()"),
        None,
        "an index"
    );
    assert_eq!(
        head(Kind::Python, "    items[0].load().delete()"),
        None,
        "an index"
    );
    assert_eq!(
        head(Kind::TsJs, "  load<Repo>(id).find()"),
        None,
        "a generic call"
    );
    assert_eq!(head(Kind::TsJs, "  if (ok).find()"), None, "a condition");
    assert_eq!(head(Kind::Python, "    repo.find()"), None, "a plain name");
    assert_eq!(
        head(Kind::TsJs, "  void café().word"),
        None,
        "a callee or a field with a character outside ASCII: no name to read"
    );
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
fn enum_constants_are_the_names_at_the_start_of_the_body() {
    let pairs = |cs: Vec<EnumConstant>| -> Vec<(String, usize)> {
        cs.into_iter().map(|c| (c.name, c.line1)).collect()
    };
    let kotlin = "enum class Lane(val f: () -> Unit = {}) : Named {\n    ROAD(\"a, b; }\"),\n    @java.lang.Deprecated AIR { override fun x() = 1 },\n    SEA,\n}";
    assert_eq!(
        pairs(enum_constants(kotlin, 1)),
        [
            ("ROAD".to_owned(), 2),
            ("AIR".to_owned(), 3),
            ("SEA".to_owned(), 4)
        ]
    );
    let java = "class A {}\n/* enum */\npublic enum Offer implements P {\n    PLAIN, // CUT,\n    /** Half. */ HALF;\n    static final int NONE = 0;\n}";
    assert_eq!(
        pairs(enum_constants(java, 3)),
        [("PLAIN".to_owned(), 4), ("HALF".to_owned(), 5)]
    );
}

#[test]
fn a_def_wrapped_over_lines_keeps_its_body_off_the_module_level() {
    let text =
        "def f(\n    a,\n):\n    import os\nclass C(\n    B\n):\n    import re\nimport sys\n";
    assert_eq!(python_module_level(text), "import sys\n");
}

#[test]
fn a_class_body_binds_for_itself_not_for_its_methods() {
    let text =
        "class C:\n    limit = 1\n    twice = limit * 2\n    def m(self):\n        return limit\n";
    assert!(python_class_binds(text, 3, "limit"));
    assert!(!python_class_binds(text, 5, "limit"));
    assert!(!python_class_binds(text, 3, "other"));
}

#[test]
fn a_keyword_argument_names_no_variable() {
    let at = |text: &str, line1: usize, word: &str| {
        let line = text.lines().nth(line1 - 1).unwrap();
        let start = line.find(word).unwrap();
        keyword_argument(text, line1, &(start..start + word.len()))
    };
    assert!(at("make(size=1)", 1, "size"));
    assert!(at("make(1, size=1)", 1, "size"));
    assert!(at("make(\n    size=1,\n)", 2, "size"));
    assert!(!at("make(size == 1)", 1, "size"));
    assert!(!at("size = 1", 1, "size"));
}
