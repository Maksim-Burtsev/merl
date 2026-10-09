use super::*;

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
        field_rows(kind, text, &hits, name, false)
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
    assert_eq!(
        found(Kind::Python, py, "repo"),
        one(8, "Issue.repo"),
        "each type's declaration once, named after the type, not the method it is assigned in"
    );
    assert_eq!(
        found(Kind::Python, py, "total"),
        vec![],
        "a local and the key of a literal are none"
    );
    assert_eq!(
        qualified(Kind::Python, py, 12, "repo").as_deref(),
        Some("Issue.repo")
    );
    let def = "def build(self):\n    self.x = 1\n";
    assert_eq!(
        found(Kind::Python, def, "x"),
        vec![],
        "outside a class `self` is a parameter like any other"
    );
    assert_eq!(
        qualified(Kind::Python, def, 2, "x").as_deref(),
        Some("build.x")
    );
    let tuple = "class Point:\n    def __init__(self):\n        self.x, self.offset = 0, 0\n\n    def move(self):\n        self.offset = 5\n";
    assert_eq!(
        found(Kind::Python, tuple, "offset"),
        one(3, "Point.offset"),
        "a tuple target is a declaration, and the later plain assignment stands for it"
    );
    let doc = "class Comment:\n    \"\"\"\n    body : str\n    \"\"\"\n\n    def __init__(self, body):\n        self.body = body\n";
    assert_eq!(
        found(Kind::Python, doc, "body"),
        one(7, "Comment.body"),
        "a docstring is none"
    );
    let ts = "export class Issue extends Base {\n  posterId = 0;\n\n  constructor(\n    private repo: Repo,\n    @Inject(Log) private log: Log,\n  ) {\n    super();\n    this.title = \"\";\n  }\n\n  resize(opts: {\n    readonly width: number;\n  }): void {\n    const box = {\n      grow() {\n        this.height = 1;\n      },\n    };\n  }\n}\nexport function tally(): void {\n  const sums = {\n    total: 0,\n  };\n  total = 2;\n}\nexport const config: {\n  readonly timeout: number;\n} = { timeout: 1 };\n";
    assert_eq!(found(Kind::TsJs, ts, "posterId"), one(2, "Issue.posterId"));
    assert_eq!(found(Kind::TsJs, ts, "repo"), one(5, "Issue.repo"));
    assert_eq!(found(Kind::TsJs, ts, "log"), one(6, "Issue.log"));
    assert_eq!(found(Kind::TsJs, ts, "title"), one(9, "Issue.title"));
    assert_eq!(found(Kind::TsJs, ts, "width"), vec![]);
    assert_eq!(found(Kind::TsJs, ts, "height"), vec![]);
    assert_eq!(
        qualified(Kind::TsJs, ts, 13, "width"),
        None,
        "a type literal's member is not named after the class"
    );
    assert_eq!(
        qualified(Kind::TsJs, ts, 17, "height"),
        None,
        "a `this` that is no class's is not named after the class"
    );
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
    assert_eq!(
        found(Kind::Go, go, "Total"),
        vec![],
        "a field of an anonymous struct is none"
    );
    assert_eq!(found(Kind::Go, go, "Host"), vec![], "a `var` block is none");
}

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
                .map(|b| b.line1)
                .collect();
            assert!(!read.is_empty(), "{kind:?}: no field {name}");
            for n in read {
                assert!(
                    re.is_match(lines[n - 1]),
                    "{kind:?}: {name}: the grep misses a line the field rules read, so the \
                     search by name would drop its type: {}",
                    lines[n - 1]
                );
            }
        }
    }
}

fn fields(kind: Kind, text: &str, decl: usize, name: &str) -> Vec<(usize, Value)> {
    field_bindings(kind, text, decl, name)
        .into_iter()
        .map(|b| (b.line1, b.value))
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
    assert_eq!(
        at("repo"),
        [
            (6, Value::Name("repo".into())),
            (16, Value::Call("make_repo".into()))
        ],
        "an argument of a call is no binding"
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
    assert_eq!(
        at("users"),
        [(3, ty("UserRepository"))],
        "the setter declares no type of its own"
    );
    assert_eq!(at("audit"), [(11, ty("\"AuditLog\""))]);
    assert_eq!(at("jobs"), [(18, ty("Jobs"))]);
    assert_eq!(at("mixins"), [(21, Value::Unknown)]);
    assert_eq!(at("plain"), []);
    assert_eq!(at("inner"), []);
    assert_eq!(
        at("nested"),
        [],
        "a property nested in a method is no field of the class"
    );
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
    assert_eq!(
        at("nested"),
        [],
        "an object literal's getter inside a method is no field of the class"
    );
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
    assert_eq!(
        at(1, "notifier"),
        [(11, Value::Name("notifier".into()))],
        "a parameter without a modifier is no field; the assignment is"
    );
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

#[test]
fn field_rows_come_in_line_order_whatever_the_order_of_the_hits() {
    let text = "class Tariff:\n    rate: int = 1\n\n\nclass Coupon:\n    rate: int = 2\n";
    assert_eq!(
        field_rows(Kind::Python, text, &[6, 2], "rate", false),
        [2, 6]
    );
}

#[test]
fn a_csharp_namespace_is_the_last_one_declared_above() {
    let text = "using System;\n\nnamespace Shop.Billing\n{\n    class Tariff {}\n}\n\nnamespace Shop.Shipping\n{\n    namespace Inner\n    {\n        class Courier {}\n    }\n}\n";
    assert_eq!(cs_namespace(text, 1), "");
    assert_eq!(cs_namespace(text, 5), "Shop.Billing");
    assert_eq!(cs_namespace(text, 9), "Shop.Shipping");
    assert_eq!(
        cs_namespace(text, 12),
        "Inner",
        "a block nested in another namespace reads under its own name only"
    );
}

#[test]
fn csharp_usings_open_namespaces_and_aliases_open_none() {
    let text = "global using Shop.Billing;\nusing Shop.Shipping ;\nusing Cut = Shop.Coupons.Cut;\nusing static Shop.Rates;\n<Using Include=\"Shop.Couriers\" />\n";
    assert_eq!(
        cs_usings(text),
        ["Shop.Billing", "Shop.Shipping", "Shop.Couriers"]
    );
}

#[test]
fn a_go_key_is_read_from_the_literal_it_stands_in() {
    let key = |text: &str, line: usize, word: &str| {
        let l = text.lines().nth(line - 1).unwrap();
        let start = l.find(&format!("{word}:")).unwrap();
        go_key(text, line, start, start + word.len())
    };
    let of = |t: &str| GoKey::Of(t.to_owned());
    let flat = "func f() {\n\tt := Tariff{Rate: 1}\n\tm := map[Rate]int{Cut: 1}\n}\n";
    assert_eq!(key(flat, 2, "Rate"), of("Tariff"));
    assert_eq!(key(flat, 3, "Cut"), GoKey::Value);
    let elided = "var all = []Tariff{\n\t{Rate: 1},\n\t{\n\t\tRate: 2,\n\t},\n}\n";
    assert_eq!(key(elided, 2, "Rate"), of("Tariff"));
    assert_eq!(
        key(elided, 4, "Rate"),
        of("Tariff"),
        "a `{{` alone on its line reads the line in front of it"
    );
    let anonymous = "var t = struct {\n\tRate int\n}{\n\tRate: 1,\n}\n";
    assert_eq!(key(anonymous, 4, "Rate"), GoKey::Struct(1));
    let raw = "var t = Tariff{\n\tNote: `\n{\n`,\n\tRate: 1,\n}\n";
    assert_eq!(
        key(raw, 5, "Rate"),
        of("Tariff"),
        "a brace inside a raw string opens nothing"
    );
    let blocks = "func f() {\n\tfor {\n\t\tdone: work()\n\t}\n\tswitch r {\n\tcase Cut:\n\t}\nretry:\n\tg()\n}\n";
    assert_eq!(key(blocks, 3, "done"), GoKey::No, "a label inside a `for`");
    assert_eq!(key(blocks, 6, "Cut"), GoKey::No);
    assert_eq!(key(blocks, 8, "retry"), GoKey::No);
    let slice = "func f() {\n\ts := xs[lo:hi]\n}\n";
    assert_eq!(key(slice, 2, "lo"), GoKey::No);
}

#[test]
fn a_python_class_body_starts_past_its_wrapped_header() {
    let text = "class Tariff(\n    Base,\n):\n    rate: int = 1\n";
    assert_eq!(fields(Kind::Python, text, 1, "rate"), [(4, ty("int"))]);
}

#[test]
fn a_jsdoc_property_is_no_field_declaration_under_the_cursor() {
    let text = "/**\n * @typedef {Object} Tariff\n * @property {number} rate\n */\n";
    let start = text.lines().nth(2).unwrap().find("rate").unwrap();
    assert!(!field_decl_at(Kind::TsJs, text, 3, start, "rate"));
}
