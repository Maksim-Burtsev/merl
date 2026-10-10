use super::*;

fn py_rows(cases: Vec<(&str, &str, &str, Shown)>) {
    for (name, file, code, want) in cases {
        let mut a = fixture_app("python");
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{name}: {file}: {code}");
    }
}

const EVERY_SEAL: [(&str, &str); 7] = [
    ("Crate.seal", "depot/crates.py:2"),
    ("Lid.seal", "depot/crates.py:7"),
    ("Pallet.seal", "depot/crates.py:12"),
    ("Tray.seal", "depot/crates.py:17"),
    ("Hook.seal", "depot/crates.py:22"),
    ("Label.seal", "depot/labels.py:5"),
    ("Pallet.seal", "depot/labels.py:10"),
];

#[test]
fn a_python_alias_and_a_module_that_hands_a_name_on_are_followed() {
    py_rows(vec![
        (
            "",
            "aliases.py",
            "repo.delete_user|(1",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "",
            "aliases.py",
            "log.delete_user|(2",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via log: AuditLog)",
                "repos.py:13",
            ),
        ),
        (
            "the alias called: the class it names",
            "aliases.py",
            "Users().find_user",
            jump(
                "find_user \u{2192} UserRepository.find_user (via Users(): UserRepository)",
                "repos.py:5",
            ),
        ),
        (
            "",
            "aliases.py",
            "repo.delete_user|(4",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "`depot/__init__.py` declares none of these",
            "aliases.py",
            "crate.seal",
            jump(
                "seal \u{2192} Crate.seal (via crate: Crate)",
                "depot/crates.py:2",
            ),
        ),
        (
            "`from .crates import Lid as Cover`",
            "aliases.py",
            "cover.seal",
            jump(
                "seal \u{2192} Lid.seal (via cover: Lid)",
                "depot/crates.py:7",
            ),
        ),
        (
            "two modules on: `depot` has it from `store`, which has it from `.sessions`",
            "aliases.py",
            "conn.close",
            jump(
                "close \u{2192} Session.close (via conn: Session)",
                "store/sessions.py:6",
            ),
        ),
        (
            "",
            "aliases.py",
            "trail.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via trail: AuditLog)",
                "repos.py:13",
            ),
        ),
        (
            "",
            "aliases.py",
            "dial().close",
            jump(
                "close \u{2192} Session.close (via dial() -> Session)",
                "store/sessions.py:6",
            ),
        ),
        (
            "`from .labels import *`",
            "aliases.py",
            "label.seal",
            jump(
                "seal \u{2192} Label.seal (via label: Label)",
                "depot/labels.py:5",
            ),
        ),
        (
            "`fakes.UserRepository`, not the one of `repos`",
            "aliases.py",
            "users.users",
            jump(
                "users \u{2192} UserRepository.users (via users: UserRepository)",
                "fakes.py:3",
            ),
        ),
        (
            "a parameter called like the alias is a value of its own type",
            "aliases.py",
            "Users.delete_user|(7",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via Users: AuditLog)",
                "repos.py:13",
            ),
        ),
        (
            "`d` on the imported word itself. `Crate` comes by two routes, the import and the `*` of a module that imports it too, to one declaration",
            "aliases.py",
            "crate: Crate",
            jump("Crate: via import depot/crates.py", "depot/crates.py:1"),
        ),
        (
            "",
            "aliases.py",
            "cover: Cover",
            jump("Cover: via import depot/crates.py", "depot/crates.py:6"),
        ),
        (
            "",
            "aliases.py",
            "conn: Session",
            jump(
                "Session: via import store/sessions.py",
                "store/sessions.py:1",
            ),
        ),
        (
            "four modules that hand the name on are followed, five are not",
            "relays.py",
            "parcel.wrap_up",
            jump(
                "wrap_up \u{2192} Parcel.wrap_up (via parcel: Parcel)",
                "relay5.py:2",
            ),
        ),
        (
            "",
            "relays.py",
            "bundle.wrap_up",
            jump(
                "wrap_up \u{2192} Parcel.wrap_up (by name, 1 match)",
                "relay5.py:2",
            ),
        ),
        (
            "an import in a docstring's example is no source",
            "docstring_import.py",
            "repo: UserRepository",
            jump("UserRepository: via import repos.py", "repos.py:4"),
        ),
        (
            "… but for the reader of that example, as it was",
            "docstring_import.py",
            "from fakes import UserRepository",
            Shown::Picker(
                "UserRepository: 2 declarations".into(),
                vec![
                    (
                        "UserRepository".into(),
                        "via import repos.py".into(),
                        "repos.py:4".into(),
                    ),
                    (
                        "UserRepository".into(),
                        "via import fakes.py".into(),
                        "fakes.py:1".into(),
                    ),
                ],
            ),
        ),
        (
            "`try` / `except ImportError` names two sources: both are offered, neither typed",
            "aliases.py",
            "pallet: Pallet",
            Shown::Picker(
                "Pallet: 2 declarations".into(),
                vec![
                    (
                        "Pallet".into(),
                        "via import depot/labels.py".into(),
                        "depot/labels.py:9".into(),
                    ),
                    (
                        "Pallet".into(),
                        "via import depot/crates.py".into(),
                        "depot/crates.py:11".into(),
                    ),
                ],
            ),
        ),
        (
            "",
            "aliases.py",
            "pallet.seal",
            picker("seal: by name, 7 declarations", &EVERY_SEAL),
        ),
        (
            "`Tray` is imported and, under an `if`, assigned",
            "aliases.py",
            "tray: Tray",
            jump("Tray: by name, 1 match", "depot/crates.py:16"),
        ),
        (
            "",
            "aliases.py",
            "tray.seal",
            picker("seal: by name, 7 declarations", &EVERY_SEAL),
        ),
        (
            "an import inside a function of the module binds nothing of the module",
            "aliases.py",
            "hook: Hook",
            jump("Hook: by name, 1 match", "depot/crates.py:21"),
        ),
        (
            "",
            "aliases.py",
            "hook.seal",
            picker("seal: by name, 7 declarations", &EVERY_SEAL),
        ),
        (
            "`depot` has `Ring` from `depot.loop`, which has it from `depot`",
            "aliases.py",
            "ring: Ring",
            jump("no definition for Ring", "aliases.py:35"),
        ),
    ]);
}

#[test]
fn a_python_class_attribute_is_looked_up_in_the_class() {
    let both_delete_user = [
        ("UserRepository.delete_user", "repos.py:8"),
        ("AuditLog.delete_user", "repos.py:13"),
    ];
    py_rows(vec![
        (
            "",
            "consts.py",
            "Limits.MAX_USERS",
            jump(
                "MAX_USERS \u{2192} Limits.MAX_USERS (via Limits)",
                "consts.py:12",
            ),
        ),
        (
            "",
            "consts.py",
            "Limits.timeout",
            jump(
                "timeout \u{2192} Limits.timeout (via Limits)",
                "consts.py:13",
            ),
        ),
        (
            "two enums with a `RED`",
            "consts.py",
            "    Color.RED",
            jump("RED \u{2192} Color.RED (via Color)", "consts.py:27"),
        ),
        (
            "",
            "consts.py",
            "    Shade.RED",
            jump("RED \u{2192} Shade.RED (via Shade)", "consts.py:32"),
        ),
        (
            "a dataclass field; `Archive.label` is a namesake",
            "consts.py",
            "Point.label",
            jump("label \u{2192} Point.label (via Point)", "consts.py:38"),
        ),
        (
            "inherited, overridden, and a method as before",
            "consts.py",
            "Tight.MAX_USERS",
            jump(
                "MAX_USERS \u{2192} Limits.MAX_USERS (via Tight)",
                "consts.py:12",
            ),
        ),
        (
            "",
            "consts.py",
            "Tight.timeout",
            jump("timeout \u{2192} Tight.timeout (via Tight)", "consts.py:20"),
        ),
        (
            "",
            "consts.py",
            "Tight.check",
            jump("check \u{2192} Limits.check (via Tight)", "consts.py:15"),
        ),
        (
            "behind an import, an alias and a module",
            "consts_use.py",
            "Color.GREEN",
            jump("GREEN \u{2192} Color.GREEN (via Color)", "consts.py:28"),
        ),
        (
            "",
            "consts_use.py",
            "Caps.MAX_USERS",
            jump(
                "MAX_USERS \u{2192} Limits.MAX_USERS (via Caps)",
                "consts.py:12",
            ),
        ),
        (
            "",
            "consts_use.py",
            "consts.Shade.RED",
            jump("RED \u{2192} Shade.RED (via consts.Shade)", "consts.py:32"),
        ),
        (
            "`self.count = 0` is an instance's",
            "consts.py",
            "Tight.count",
            jump(
                "count \u{2192} Tight.count (by name, 1 match)",
                "consts.py:23",
            ),
        ),
        (
            "",
            "consts.py",
            "Limits.missing",
            jump("no definition for missing", "consts.py:48"),
        ),
        (
            "",
            "consts.py",
            "Color.RED.value",
            jump(
                "no definition for value (chain broke at Color)",
                "consts.py:53",
            ),
        ),
        (
            "",
            "consts.py",
            "scan.cache",
            jump("no definition for cache", "consts.py:54"),
        ),
        (
            "the function's own `class Color`, which the rules do not read",
            "consts.py",
            "Color.RED|  # the class",
            picker(
                "RED: by name, 3 declarations",
                &[
                    ("Color.RED", "consts.py:27"),
                    ("Shade.RED", "consts.py:32"),
                    ("inner.Color.RED", "consts.py:59"),
                ],
            ),
        ),
        (
            "",
            "consts.py",
            "Color.RED|  # a parameter",
            jump("RED \u{2192} Shade.RED (via Color: Shade)", "consts.py:32"),
        ),
        (
            "",
            "consts.py",
            "Limits.MAX_USERS|  # a parameter",
            jump(
                "MAX_USERS \u{2192} Limits.MAX_USERS (by name, 1 match)",
                "consts.py:12",
            ),
        ),
        (
            "the class binds the word in a shape the rules do not read: a tuple, a `def` under an `if`, a `for`. Not reading it is no proof that the base's is meant",
            "consts.py",
            "    Unread.RANK",
            jump(
                "RANK \u{2192} Plain.RANK (by name, 1 match)",
                "consts.py:101",
            ),
        ),
        (
            "under an `if` the walk of `qualified` names nothing",
            "consts.py",
            "    Unread.check",
            picker(
                "check: by name, 3 declarations",
                &[
                    ("Limits.check", "consts.py:15"),
                    ("Plain.check", "consts.py:105"),
                    ("check", "consts.py:112"),
                ],
            ),
        ),
        (
            "",
            "consts.py",
            "    Unread.CODE",
            jump(
                "CODE \u{2192} Plain.CODE (by name, 1 match)",
                "consts.py:103",
            ),
        ),
        (
            "the body goes on past a comment and a string at column 0, and may share the header's line",
            "consts.py",
            "    Noted.Meta",
            jump("Meta \u{2192} Noted.Meta (via Noted)", "consts.py:133"),
        ),
        (
            "",
            "consts.py",
            "    Queried.RANK",
            jump(
                "RANK \u{2192} Plain.RANK (by name, 1 match)",
                "consts.py:101",
            ),
        ),
        (
            "",
            "consts.py",
            "    Short.CODE",
            jump(
                "CODE \u{2192} Plain.CODE (by name, 1 match)",
                "consts.py:103",
            ),
        ),
        (
            "the header's own lines at the margin end nothing either",
            "consts.py",
            "    Flush.CODE",
            jump(
                "CODE \u{2192} Plain.CODE (by name, 1 match)",
                "consts.py:103",
            ),
        ),
        (
            "… and a nested class, which is what the class declares",
            "consts.py",
            "    Unread.Meta",
            jump("Meta \u{2192} Unread.Meta (via Unread)", "consts.py:115"),
        ),
        (
            "two bases that disagree: the order Python reads them in is not computed",
            "consts.py",
            "Diamond.LEVEL",
            picker(
                "LEVEL: by name, 2 declarations",
                &[
                    ("Root.LEVEL", "consts.py:80"),
                    ("Right.LEVEL", "consts.py:88"),
                ],
            ),
        ),
        (
            "`with … as`: one line, two targets, and wrapped in brackets",
            "consts.py",
            "        conn|.delete",
            jump("conn \u{2192} scan.conn (local)", "consts.py:70"),
        ),
        (
            "",
            "consts.py",
            "        handle|.read",
            jump("handle \u{2192} scan.handle (local)", "consts.py:70"),
        ),
        (
            "",
            "consts.py",
            "        wrapped|.delete",
            jump("wrapped: local", "consts.py:74"),
        ),
        (
            "",
            "consts.py",
            "conn.delete_user",
            picker("delete_user: by name, 2 declarations", &both_delete_user),
        ),
        (
            "",
            "consts.py",
            "wrapped.delete_user",
            picker("delete_user: by name, 2 declarations", &both_delete_user),
        ),
    ]);
}

#[test]
fn a_wrapped_python_class_header_keeps_its_name_and_its_bases() {
    py_rows(vec![
        (
            "its members are the class's",
            "impls.py",
            "self.mop",
            jump(
                "mop \u{2192} WrappedJob.mop (via self: WrappedJob)",
                "impls.py:60",
            ),
        ),
        (
            "its members are the class's",
            "impls.py",
            "job.mop",
            jump(
                "mop \u{2192} WrappedJob.mop (via job: WrappedJob)",
                "impls.py:60",
            ),
        ),
        (
            "from inside it a member's implementations are found, the bases of the implementing class sharing one line under theirs",
            "impls.py",
            "def tick",
            jump(
                "tick \u{2192} Narrow.tick (implementations of WideBase.tick)",
                "impls.py:94",
            ),
        ),
        (
            "its bases are read",
            "impls.py",
            "self.sweep",
            jump(
                "sweep \u{2192} Sweeper.sweep (via self: Narrow)",
                "impls.py:44",
            ),
        ),
    ]);
}

#[test]
fn a_python_parameter_is_named_after_its_function() {
    py_rows(vec![
        (
            "",
            "params.py",
            "found = slug",
            jump(
                "slug \u{2192} RecipeController.get_one.slug (local)",
                "params.py:2",
            ),
        ),
        (
            "",
            "params.py",
            "return found",
            jump(
                "found \u{2192} RecipeController.get_one.found (local)",
                "params.py:3",
            ),
        ),
        (
            "a signature wrapped over several lines: the parameter, and a local under its `)`",
            "params.py",
            "kept = slugs",
            jump(
                "slugs \u{2192} RecipeController.get_many.slugs (local)",
                "params.py:8",
            ),
        ),
        (
            "",
            "params.py",
            "return kept",
            jump(
                "kept \u{2192} RecipeController.get_many.kept (local)",
                "params.py:10",
            ),
        ),
        (
            "",
            "params.py",
            "        return slug",
            jump(
                "slug \u{2192} RecipeController.get_later.slug (local)",
                "params.py:13",
            ),
        ),
        (
            "",
            "params.py",
            "return level",
            jump("level \u{2192} top.level (local)", "params.py:17"),
        ),
    ]);
}

#[test]
fn a_python_binding_need_not_start_its_line() {
    let users = |name: &'static str, n: &str| {
        (
            name,
            "scopes.py",
            format!("ledger.delete_user|({n}"),
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via ledger: UserRepository)",
                "repos.py:8",
            ),
        )
    };
    let unproven = |name: &'static str, n: &str, status: &str| {
        let rows = [
            ("UserRepository.delete_user", "repos.py:8"),
            ("AuditLog.delete_user", "repos.py:13"),
        ];
        (
            name,
            "scopes.py",
            format!("ledger.delete_user|({n}"),
            picker(status, &rows),
        )
    };
    let module = |name: &'static str, n: &str| {
        (
            name,
            "scopes.py",
            format!("ledger.delete_user|({n}"),
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via ledger: AuditLog)",
                "repos.py:13",
            ),
        )
    };
    let by_name = "delete_user: by name, 2 declarations";
    let cases = vec![
        users("`if fresh: ledger = UserRepository()`", "10"),
        unproven("… and `else: ledger = open(\"ledger\")`", "11", by_name),
        users("`count = 1; ledger = UserRepository()`", "12 + count"),
        users(
            "`for name in names[\"a:b\"]: ledger = …`: the `:` of the string ends no header",
            "13",
        ),
        users("`with … as source: ledger: UserRepository = source`", "14"),
        users(
            "the `:` of a header wrapped over two lines, the first ending in `and`",
            "19",
        ),
        users(
            "the `:` of a header wrapped over two lines, the first ending in `(`",
            "20",
        ),
        unproven("`first = ledger = UserRepository()`", "15 + len", by_name),
        unproven(
            "`try: from fakes import ledger`: only the statement starts with `from`",
            "16",
            by_name,
        ),
        unproven(
            "`if cold: self.ledger = UserRepository()` beside `self.ledger = AuditLog()`",
            "18",
            "delete_user: by name, 2 declarations (chain broke at ledger)",
        ),
        module(
            "`if ledger == flag: print(ledger)` binds nothing: the module's",
            "17",
        ),
        module(
            "a keyword argument on a line that continues a call binds nothing: the module's",
            "21",
        ),
        module(
            "a lambda's `:` behind the end of a call's arguments is no header's",
            "22",
        ),
    ];
    for (name, file, code, want) in cases {
        let mut a = fixture_app("python");
        d_on(&mut a, file, &code);
        assert_eq!(shown(&mut a), want, "{name}: {file}: {code}");
    }
}

#[test]
fn an_import_inside_the_project_is_looked_up_in_its_module() {
    let cases: [(&str, &str, &str, &str, Shown); 13] = [
        (
            "`fakes.py` declares a `UserRepository` too",
            "python",
            "jobs.py",
            "repo: UserRepository",
            jump("UserRepository: via import repos.py", "repos.py:4"),
        ),
        (
            "a package is its `__init__.py`",
            "python",
            "jobs.py",
            "    connect",
            jump(
                "connect: via import store/__init__.py",
                "store/__init__.py:6",
            ),
        ),
        (
            "`store/__init__.py` imports it from `.sessions` and hands it on",
            "python",
            "jobs.py",
            "    open_session",
            jump(
                "open_session: via import store/sessions.py",
                "store/sessions.py:16",
            ),
        ),
        (
            "",
            "python",
            "jobs.py",
            "sessions.open_session",
            jump(
                "open_session: via import store/sessions.py",
                "store/sessions.py:16",
            ),
        ),
        (
            "through an aliased class: its own `start`, not `Pool.start` in the same module",
            "python",
            "jobs.py",
            "StoreSession.start",
            jump(
                "start \u{2192} Session.start (via import store/sessions.py)",
                "store/sessions.py:3",
            ),
        ),
        (
            "",
            "python",
            "store/__init__.py",
            "return open_session",
            jump(
                "open_session: via import store/sessions.py",
                "store/sessions.py:16",
            ),
        ),
        (
            "",
            "typescript",
            "jobs.ts",
            "repo: UserRepository",
            jump("UserRepository: via import repos.ts", "repos.ts:5"),
        ),
        (
            "a default import under another name: the module's `export default`",
            "typescript",
            "jobs.ts",
            "  connectToStore",
            jump(
                "connectToStore: via import store/index.ts",
                "store/index.ts:5",
            ),
        ),
        (
            "",
            "typescript",
            "jobs.ts",
            "  openSession",
            picker(
                "openSession: by name, 2 declarations",
                &[
                    ("openSession", "fakes.ts:5"),
                    ("openSession", "store/sessions.ts:15"),
                ],
            ),
        ),
        (
            "a namespace behind the `@/` alias of tsconfig.json",
            "typescript",
            "jobs.ts",
            "sessions.openSession",
            jump(
                "openSession: via import store/sessions.ts",
                "store/sessions.ts:15",
            ),
        ),
        (
            "",
            "typescript",
            "jobs.ts",
            "StoreSession.start",
            jump(
                "start \u{2192} Session.start (via import store/sessions.ts)",
                "store/sessions.ts:2",
            ),
        ),
        (
            "",
            "typescript",
            "store/index.ts",
            "return openSession",
            jump(
                "openSession: via import store/sessions.ts",
                "store/sessions.ts:15",
            ),
        ),
        (
            "the package function, not the method `Session.Open` nor `fakes.Open`",
            "go",
            "jobs.go",
            "store.Open",
            jump("Open: via import store/", "store/store.go:7"),
        ),
    ];
    for (name, fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{name}: {fixture}: {code}");
    }
}

#[test]
fn a_typed_receiver_finds_what_its_type_extends() {
    type Probe = (&'static str, &'static str, Shown);
    type Project = (Kind, &'static [(&'static str, &'static str)], Vec<Probe>);
    let projects: [Project; 3] = [
        (
            Kind::Python,
            &[
                (
                    "base.py",
                    "class BaseRepo:\n    def find(self, key):\n        pass\n",
                ),
                (
                    "repos.py",
                    "from base import BaseRepo\n\n\nclass UserRepo(BaseRepo):\n    pass\n\n\nclass Other:\n    def find(self, key):\n        pass\n",
                ),
                (
                    "main.py",
                    "from repos import UserRepo\n\n\ndef run(repo: UserRepo):\n    repo.find(1)\n\n\nclass BaseService:\n    def __init__(self, repo: UserRepo):\n        self.repo = repo\n\n\nclass Service(BaseService):\n    def run(self):\n        self.repo.find(1)\n",
                ),
            ],
            vec![
                (
                    "main.py",
                    "^    repo.find",
                    jump(
                        "find \u{2192} BaseRepo.find (via repo: UserRepo)",
                        "base.py:2",
                    ),
                ),
                (
                    "main.py",
                    "self.repo.find",
                    jump(
                        "find \u{2192} BaseRepo.find (via self.repo: UserRepo)",
                        "base.py:2",
                    ),
                ),
            ],
        ),
        (
            Kind::TsJs,
            &[
                (
                    "base.ts",
                    "export class BaseRepo {\n  find(key: string): void {}\n}\n",
                ),
                (
                    "repos.ts",
                    "import { BaseRepo } from \"./base\";\n\nexport class UserRepo extends BaseRepo {}\n\nexport class Other {\n  find(key: string): void {}\n}\n",
                ),
                (
                    "main.ts",
                    "import { UserRepo } from \"./repos\";\n\nexport function run(repo: UserRepo): void {\n  repo.find(\"a\");\n}\n",
                ),
            ],
            vec![(
                "main.ts",
                "repo.find",
                jump(
                    "find \u{2192} BaseRepo.find (via repo: UserRepo)",
                    "base.ts:2",
                ),
            )],
        ),
        (
            Kind::Go,
            &[
                ("go.mod", "module example.com/inherit\n"),
                (
                    "base.go",
                    "package main\n\ntype Base struct{}\n\nfunc (b *Base) Find(key string) {}\n",
                ),
                (
                    "repos.go",
                    "package main\n\ntype UserRepo struct {\n\t*Base\n\tname string\n}\n\ntype Other struct{}\n\nfunc (o Other) Find(key string) {}\n",
                ),
                (
                    "main.go",
                    "package main\n\nfunc run(repo *UserRepo) {\n\trepo.Find(\"a\")\n}\n",
                ),
            ],
            vec![(
                "main.go",
                "repo.Find",
                jump("Find \u{2192} Base.Find (via repo: UserRepo)", "base.go:5"),
            )],
        ),
    ];
    for (kind, files, probes) in projects {
        let (dir, mut a) = project_app(&format!("extends-{kind:?}"), files);
        a.external.insert(kind, (Vec::new(), Arc::new(Vec::new())));
        for (file, code, want) in probes {
            d_on(&mut a, file, code);
            assert_eq!(shown(&mut a), want, "{file}: {code}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn a_member_of_a_value_is_looked_for_outside_the_project_too() {
    let mut a = fixture_app("python");
    let site = std::env::temp_dir().join(format!("merl-site-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&site);
    std::fs::create_dir_all(site.join("client")).unwrap();
    std::fs::write(
        site.join("client/api.py"),
        "class Client:\n    def find_user(self, user_id):\n        pass\n",
    )
    .unwrap();
    a.external.insert(
        Kind::Python,
        (
            vec![site.clone()],
            Arc::new(vec![site.join("client/api.py")]),
        ),
    );
    d_on(&mut a, "factories.py", "repo.find_user");
    assert_eq!(
        shown(&mut a),
        picker(
            "find_user: by name, 2 declarations",
            &[
                ("UserRepository.find_user", "repos.py:5"),
                ("Client.find_user", "client/api.py:2"),
            ],
        ),
        "the project first; a dependency shown relative to its root"
    );
    let mut a = fixture_app("typescript");
    let (dts, js) = (site.join("client/index.d.ts"), site.join("client/index.js"));
    std::fs::write(
        &dts,
        "export declare class Client {\n    findUser(id: number): unknown;\n}\n",
    )
    .unwrap();
    std::fs::write(&js, "class Client {\n  findUser(id) {\n  }\n}\n").unwrap();
    a.external
        .insert(Kind::TsJs, (vec![site.clone()], Arc::new(vec![dts, js])));
    d_on(&mut a, "factories.ts", "repo.findUser");
    assert_eq!(
        shown(&mut a),
        picker(
            "findUser: by name, 2 declarations",
            &[
                ("UserRepository.findUser", "repos.ts:6"),
                ("Client.findUser", "client/index.d.ts:2"),
            ],
        ),
        "TypeScript outside the project is read from its declarations, not the bundle"
    );
    std::fs::remove_dir_all(&site).unwrap();
}

#[test]
fn a_module_name_in_or_bound_by_an_import_opens_the_module() {
    let std = external_root(
        "py-modules-std",
        &[
            ("json/__init__.py", "def dumps(obj):\n    pass\n"),
            (
                "site-packages/pip/_internal/cli/cmdoptions.py",
                "json = object()\n",
            ),
        ],
    );
    let site = external_root(
        "py-modules-site",
        &[
            ("django/__init__.py", ""),
            ("django/core/__init__.py", ""),
            (
                "django/core/management/__init__.py",
                "class CommandError(Exception):\n    pass\n",
            ),
            ("rest_framework/__init__.py", ""),
            (
                "rest_framework/serializers.py",
                "class ListField:\n    pass\n",
            ),
            (
                "fsspec/github.py",
                "class GithubFileSystem:\n    def repos(self):\n        pass\n",
            ),
            ("kombu/utils/yaml.py", "def load(s):\n    pass\n"),
            ("drf/__init__.py", "fields = None\n"),
            ("drf/fields.py", "x = 1\n"),
        ],
    );
    let (dir, mut a) = project_app(
        "py-modules",
        &[
            ("app/__init__.py", ""),
            ("app/repos.py", "class UserRepo:\n    pass\n"),
            (
                "app/modules.py",
                "from app.repos import UserRepo\nfrom django.core.management import CommandError\nimport json\nfrom rest_framework import serializers\nimport yaml\nfrom drf import fields\n\n\ndef main():\n    json.dumps({})\n    serializers.ListField()\n    yaml.load(\"\")\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Python, &[std.clone(), site.clone()]);
    let module = |word: &str, file: &str, root: &Path| {
        let place = format!("{}:1", root.join(file).display());
        jump(&format!("{word}: module {file}"), &place)
    };
    for (name, code, want) in [
        (
            "",
            "from app.repos",
            module("repos", "app/repos.py", Path::new("")),
        ),
        (
            "",
            "from app|.repos",
            module("app", "app/__init__.py", Path::new("")),
        ),
        (
            "",
            "from django.core.management",
            module("management", "django/core/management/__init__.py", &site),
        ),
        (
            "",
            "from django.core|.management",
            module("core", "django/core/__init__.py", &site),
        ),
        (
            "the base interpreter's pip, which a module found by name used to land in, is no answer",
            "^import json",
            module("json", "json/__init__.py", &std),
        ),
        ("", "json|.dumps", module("json", "json/__init__.py", &std)),
        (
            "",
            "import serializers",
            module("serializers", "rest_framework/serializers.py", &site),
        ),
        (
            "",
            "serializers|.ListField",
            module("serializers", "rest_framework/serializers.py", &site),
        ),
        (
            "nothing of the name at the root: no module, and no search by name; a module of the name deeper in another package is not the one imported",
            "^import yaml",
            jump("no definition for yaml", "app/modules.py:5"),
        ),
        (
            "nothing of the name at the root: no module, and no search by name",
            "^    yaml|.load",
            jump("no definition for yaml", "app/modules.py:12"),
        ),
        (
            "unchanged: what the import takes",
            "import UserRepo",
            jump("UserRepo: via import app/repos.py", "app/repos.py:1"),
        ),
        (
            "",
            "json.dumps",
            jump(
                "dumps: via import json",
                &format!("{}:1", std.join("json/__init__.py").display()),
            ),
        ),
        (
            "",
            "serializers.ListField",
            jump(
                "ListField: via import rest_framework.serializers",
                &format!("{}:1", site.join("rest_framework/serializers.py").display()),
            ),
        ),
        (
            "unchanged: a name the package above binds, a package that binds the name itself keeps its say",
            "import fields",
            jump(
                "fields: by name, 1 match",
                &format!("{}:1", site.join("drf/__init__.py").display()),
            ),
        ),
    ] {
        d_on(&mut a, "app/modules.py", code);
        assert_eq!(shown(&mut a), want, "{name}: {code}");
    }
    for d in [dir, std, site] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_member_from_outside_never_lands_on_a_namesake() {
    let std = external_root(
        "py-outside-std",
        &[(
            "unittest/mock.py",
            "class NonCallableMock:\n    return_value = property(lambda self: None)\n\n\nclass Mock(NonCallableMock):\n    pass\n",
        )],
    );
    let site = external_root(
        "py-outside-site",
        &[
            (
                "django/test/__init__.py",
                "from django.test.testcases import TestCase\n",
            ),
            (
                "django/test/testcases.py",
                "import unittest\n\n\nclass SimpleTestCase(unittest.TestCase):\n    client = None\n\n\nclass TestCase(SimpleTestCase):\n    pass\n",
            ),
            (
                "django/contrib/auth/models.py",
                "class AbstractUser:\n    objects = None\n\n\nclass User(AbstractUser):\n    pass\n",
            ),
            ("nltk/chomsky.py", "objects = \"text\"\n"),
            (
                "django/conf/__init__.py",
                "class LazySettings:\n    def __getattr__(self, name):\n        pass\n\n\nsettings = LazySettings()\n",
            ),
            (
                "django/conf/global_settings.py",
                "ORIGINALS_DIR = None\nAUTH_USER_MODEL = \"auth.User\"\n",
            ),
            (
                "otherlib/consts.py",
                "ORIGINALS_DIR = 1\nAUTH_USER_MODEL = 1\n",
            ),
            (
                "condpkg/models.py",
                "if True:\n    class Thing:\n        objects = None\n",
            ),
            (
                "anyio/tasks.py",
                "class TaskHandle:\n    def return_value(self):\n        pass\n\n    def captured_queries(self):\n        pass\n",
            ),
            ("six.py", "def with_metaclass(meta, *bases):\n    pass\n"),
        ],
    );
    let (dir, mut a) = project_app(
        "py-outside",
        &[
            (
                "app/mailer.py",
                "class Mailer:\n    def __init__(self, client):\n        self.client = client\n",
            ),
            (
                "app/test_views.py",
                "from django.test import TestCase\n\n\nclass TestViews(TestCase):\n    def test_get(self) -> None:\n        self.client.get(\"/\")\n",
            ),
            (
                "app/users.py",
                "from django.contrib.auth.models import User\n\n\ndef users():\n    return User.objects.all()\n",
            ),
            (
                "app/mocks.py",
                "from unittest import mock\n\n\ndef use(m: mock.Mock) -> None:\n    m.return_value = None\n    m.captured_queries()\n",
            ),
            (
                "app/settings.py",
                "ORIGINALS_DIR = \"originals\"\nclient = None\nif True:\n    CACHE_DIR = \"cache\"\n",
            ),
            (
                "app/consumers.py",
                "class Consumer:\n    def connect(self):\n        pass\n",
            ),
            (
                "app/things.py",
                "from condpkg.models import Thing\nfrom django.test import TestCase\n\n\ndef things():\n    return Thing.objects, TestCase.client\n",
            ),
            (
                "app/test_files.py",
                "from django.conf import settings\n\n\ndef originals():\n    settings.connect()\n    return settings.ORIGINALS_DIR, settings.AUTH_USER_MODEL\n\n\ndef cache():\n    return settings.CACHE_DIR\n",
            ),
            (
                "app/test_api.py",
                "from django.test import TestCase\n\n\nclass ApiCase(TestCase):\n    def test_post(self) -> None:\n        self.client.post(\"/\")\n\n\nclass SignedCase(ApiCase):\n    def setUp(self) -> None:\n        self.client = None\n",
            ),
            (
                "app/test_native.py",
                "from fastbase import Base\n\n\nclass NativeCase(Base):\n    def test_post(self) -> None:\n        self.client.post(\"/\")\n\n\nclass SignedNative(NativeCase):\n    def setUp(self) -> None:\n        self.client = None\n",
            ),
            (
                "app/mocks_native.py",
                "from unittest import mock\n\n\ndef use(m: mock.NativeMock) -> None:\n    m.return_value = None\n",
            ),
            (
                "app/test_six.py",
                "import six\nfrom django.test import TestCase\n\n\nclass SixCase(six.with_metaclass(type, TestCase)):\n    def test_put(self) -> None:\n        self.client.put(\"/\")\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Python, &[std.clone(), site.clone()]);
    let outside = |root: &Path, file: &str| format!("{}", root.join(file).display());
    for (name, file, code, want) in [
        (
            "",
            "app/test_views.py",
            "self.client",
            jump(
                "client \u{2192} SimpleTestCase.client (via self: TestViews)",
                &format!("{}:5", outside(&site, "django/test/testcases.py")),
            ),
        ),
        (
            "a name qualified by a class imported from outside is a member of that class, never a top-level namesake",
            "app/users.py",
            "User.objects",
            jump(
                "objects \u{2192} AbstractUser.objects (via import django.contrib.auth.models)",
                &format!("{}:2", outside(&site, "django/contrib/auth/models.py")),
            ),
        ),
        (
            "Django's `settings` is an instance whose `__getattr__` reads the project's settings module: not a class of the module, what it holds is not read, the search by name answers; namesakes outside, in the module's package and elsewhere, are no setting of the project",
            "app/test_files.py",
            "settings.ORIGINALS_DIR",
            jump("ORIGINALS_DIR: by name, 1 match", "app/settings.py:1"),
        ),
        (
            "what the project does not set is Django's default, never a namesake elsewhere",
            "app/test_files.py",
            "settings.AUTH_USER_MODEL",
            jump(
                "AUTH_USER_MODEL: via import django.conf",
                &format!("{}:2", outside(&site, "django/conf/global_settings.py")),
            ),
        ),
        (
            "only a module-level assignment of the project: never a method of a project class",
            "app/test_files.py",
            "settings.connect",
            jump("no definition for connect", "app/test_files.py:5"),
        ),
        (
            "a setting assigned under a module-level `if` is not read (#792)",
            "app/test_files.py",
            "settings.CACHE_DIR",
            jump("no definition for CACHE_DIR", "app/test_files.py:10"),
        ),
        (
            "a class declared under an `if` is a class of the module all the same",
            "app/things.py",
            "Thing.objects",
            jump(
                "objects \u{2192} Thing.objects (via import condpkg.models)",
                &format!("{}:3", outside(&site, "condpkg/models.py")),
            ),
        ),
        (
            "",
            "app/things.py",
            "TestCase.client",
            jump(
                "client \u{2192} SimpleTestCase.client (via import django.test)",
                &format!("{}:5", outside(&site, "django/test/testcases.py")),
            ),
        ),
        (
            "",
            "app/mocks.py",
            "m.return_value",
            jump(
                "return_value \u{2192} NonCallableMock.return_value (via m: Mock)",
                &format!("{}:2", outside(&std, "unittest/mock.py")),
            ),
        ),
        (
            "",
            "app/mocks.py",
            "m.captured_queries",
            jump("no definition for captured_queries", "app/mocks.py:6"),
        ),
        (
            "",
            "app/test_api.py",
            "self.client|.post",
            jump(
                "client \u{2192} SimpleTestCase.client (via self: ApiCase)",
                &format!("{}:5", outside(&site, "django/test/testcases.py")),
            ),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{name}: {file}: {code}");
    }
    d_on(&mut a, "app/test_native.py", "self.client|.post");
    assert_eq!(
        shown(&mut a),
        jump(
            "client \u{2192} SignedNative.client (by name, 1 match)",
            "app/test_native.py:11"
        ),
        "a member a project class lacks, when its ancestry goes outside the project, is looked for only in the project classes extending it"
    );
    d_on(&mut a, "app/mocks_native.py", "m.return_value");
    assert_eq!(
        shown(&mut a),
        Shown::Picker(
            "return_value: by name, 1+ declarations".into(),
            vec![(
                "TaskHandle.return_value".into(),
                "by name".into(),
                "anyio/tasks.py:2".into(),
            )],
        ),
        "one method outside is offered, not jumped to, while a field of the name is declared outside too"
    );
    d_on(&mut a, "app/test_six.py", "self.client");
    let Shown::Picker(_, rows) = shown(&mut a) else {
        panic!("a picker");
    };
    assert!(
        rows.iter().any(|r| r.2 == "app/mailer.py:3"),
        "a base written as a call cannot be read: today's search by name: {rows:?}"
    );
    for d in [dir, std, site] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_builtin_says_it_has_no_source() {
    let std = external_root(
        "py-builtins-std",
        &[
            (
                "ast.py",
                "class NodeVisitor:\n    def next(self):\n        pass\n",
            ),
            (
                "datetime.py",
                "class date:\n    def replace(self, year=None):\n        pass\n",
            ),
            ("tools/__init__.py", "def helper():\n    pass\n"),
            (
                "other.py",
                "class Thing:\n    def helper(self):\n        pass\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "py-builtins",
        &[
            (
                "app/builtins_use.py",
                "def render() -> str | None:\n    return None\n\n\ndef go(name: str) -> None:\n    s = render()\n    s.replace(\"a\", \"b\")\n    first = next(iter([1]))\n    name.upper()\n    helper()\n",
            ),
            (
                "app/starred.py",
                "from tools import *\n\n\ndef go() -> None:\n    helper()\n",
            ),
            (
                "app/own_str.py",
                "class str:\n    def replace(self):\n        pass\n\n\ndef go(s: str) -> None:\n    s.replace()\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&std));
    let here = |file: &str, line: usize| format!("{file}:{line}");
    for (name, file, code, want) in [
        (
            "a member of a value proven to be a `str` says it has no source rather than jumping to a namesake outside",
            "app/builtins_use.py",
            "s.replace",
            jump(
                "replace: builtin, no source (via render() -> str)",
                &here("app/builtins_use.py", 7),
            ),
        ),
        (
            "a bare `next` nothing in the file binds says it has no source rather than jumping to a namesake outside",
            "app/builtins_use.py",
            "next",
            jump("next: builtin, no source", &here("app/builtins_use.py", 8)),
        ),
        (
            "",
            "app/builtins_use.py",
            "name.upper",
            jump(
                "upper: builtin, no source (via name: str)",
                &here("app/builtins_use.py", 9),
            ),
        ),
        (
            "a bare name nothing binds that is no builtin is no method outside",
            "app/builtins_use.py",
            "^    helper",
            jump("no definition for helper", &here("app/builtins_use.py", 10)),
        ),
        (
            "only a module the file `*`-imports can declare a bare name outside",
            "app/starred.py",
            "^    helper",
            jump(
                "helper: via import tools",
                &format!("{}:1", std.join("tools/__init__.py").display()),
            ),
        ),
        (
            "a project class called `str` is read as before",
            "app/own_str.py",
            "s.replace",
            jump(
                "replace \u{2192} str.replace (via s: str)",
                &here("app/own_str.py", 2),
            ),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{name}: {file}: {code}");
    }
    std::fs::remove_dir_all(dir).unwrap();
    std::fs::remove_dir_all(std).unwrap();
}

#[test]
fn a_python_def_nested_in_a_function_outside_is_no_method() {
    let std = external_root(
        "py-338",
        &[(
            "mock.py",
            "def _setup_func(funcopy, mock):\n    def assert_not_called():\n        return mock.assert_not_called()\n\n    funcopy.assert_not_called = assert_not_called\n\n\nclass NonCallableMock:\n    def assert_not_called(self):\n        pass\n",
        )],
    );
    let (dir, mut a) = project_app(
        "py-338",
        &[(
            "app/spy.py",
            "def check(spy):\n    spy.assert_not_called()\n",
        )],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&std));
    d_on(&mut a, "app/spy.py", "spy.assert_not_called");
    assert_eq!(
        shown(&mut a),
        jump(
            "assert_not_called \u{2192} NonCallableMock.assert_not_called (by name, 1 match)",
            &format!("{}:9", std.join("mock.py").display()),
        ),
        "`unittest/mock.py` nests a `def assert_not_called` in `_setup_func`, beside the method of that name"
    );
    std::fs::remove_dir_all(dir).unwrap();
    std::fs::remove_dir_all(std).unwrap();
}
