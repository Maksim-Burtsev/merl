//! What a name is bound to around the cursor.

use super::*;

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
