//! `d` in Python.

use super::*;

fn py_rows(cases: Vec<(&str, &str, Shown)>) {
    for (file, code, want) in cases {
        let mut a = fixture_app("python");
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{file}: {code}");
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

/// #100. `from x import y as z` types a receiver as `y`, and a module of the project that
/// imports a name without declaring it hands it on: a package's `__init__.py`, by a relative
/// import, under another name, from another package that hands it on in turn, through
/// `import *`. What the module may not end up with stays by name: two sources, a name it
/// also assigns, the import of a function in it, a cycle.
#[test]
fn a_python_alias_and_a_module_that_hands_a_name_on_are_followed() {
    py_rows(vec![
        (
            "aliases.py",
            "repo.delete_user|(1",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "aliases.py",
            "log.delete_user|(2",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via log: AuditLog)",
                "repos.py:13",
            ),
        ),
        // The alias called: the class it names.
        (
            "aliases.py",
            "Users().find_user",
            jump(
                "find_user \u{2192} UserRepository.find_user (via Users(): UserRepository)",
                "repos.py:5",
            ),
        ),
        (
            "aliases.py",
            "repo.delete_user|(4",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                "repos.py:8",
            ),
        ),
        // `depot/__init__.py` declares none of these.
        (
            "aliases.py",
            "crate.seal",
            jump(
                "seal \u{2192} Crate.seal (via crate: Crate)",
                "depot/crates.py:2",
            ),
        ),
        // `from .crates import Lid as Cover`.
        (
            "aliases.py",
            "cover.seal",
            jump(
                "seal \u{2192} Lid.seal (via cover: Lid)",
                "depot/crates.py:7",
            ),
        ),
        // Two modules on: `depot` has it from `store`, which has it from `.sessions`.
        (
            "aliases.py",
            "conn.close",
            jump(
                "close \u{2192} Session.close (via conn: Session)",
                "store/sessions.py:6",
            ),
        ),
        (
            "aliases.py",
            "trail.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via trail: AuditLog)",
                "repos.py:13",
            ),
        ),
        (
            "aliases.py",
            "dial().close",
            jump(
                "close \u{2192} Session.close (via dial() -> Session)",
                "store/sessions.py:6",
            ),
        ),
        // `from .labels import *`.
        (
            "aliases.py",
            "label.seal",
            jump(
                "seal \u{2192} Label.seal (via label: Label)",
                "depot/labels.py:5",
            ),
        ),
        // `fakes.UserRepository`, not the one of `repos`.
        (
            "aliases.py",
            "users.users",
            jump(
                "users \u{2192} UserRepository.users (via users: UserRepository)",
                "fakes.py:3",
            ),
        ),
        // A parameter called like the alias is a value of its own type.
        (
            "aliases.py",
            "Users.delete_user|(7",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via Users: AuditLog)",
                "repos.py:13",
            ),
        ),
        // `d` on the imported word itself. `Crate` comes by two routes, the import and
        // the `*` of a module that imports it too, to one declaration.
        (
            "aliases.py",
            "crate: Crate",
            jump("Crate: via import depot/crates.py", "depot/crates.py:1"),
        ),
        (
            "aliases.py",
            "cover: Cover",
            jump("Cover: via import depot/crates.py", "depot/crates.py:6"),
        ),
        (
            "aliases.py",
            "conn: Session",
            jump(
                "Session: via import store/sessions.py",
                "store/sessions.py:1",
            ),
        ),
        // Four modules that hand the name on are followed, five are not.
        (
            "relays.py",
            "parcel.wrap_up",
            jump(
                "wrap_up \u{2192} Parcel.wrap_up (via parcel: Parcel)",
                "relay5.py:2",
            ),
        ),
        (
            "relays.py",
            "bundle.wrap_up",
            jump(
                "wrap_up \u{2192} Parcel.wrap_up (by name, 1 match)",
                "relay5.py:2",
            ),
        ),
        // An import in a docstring's example is no source.
        (
            "docstring_import.py",
            "repo: UserRepository",
            jump("UserRepository: via import repos.py", "repos.py:4"),
        ),
        // … but for the reader of that example, as it was.
        (
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
        // `try` / `except ImportError` names two sources: both are offered, neither typed.
        (
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
            "aliases.py",
            "pallet.seal",
            picker("seal: by name, 7 declarations", &EVERY_SEAL),
        ),
        // `Tray` is imported and, under an `if`, assigned.
        (
            "aliases.py",
            "tray: Tray",
            jump("Tray: by name, 1 match", "depot/crates.py:16"),
        ),
        (
            "aliases.py",
            "tray.seal",
            picker("seal: by name, 7 declarations", &EVERY_SEAL),
        ),
        // An import inside a function of the module binds nothing of the module.
        (
            "aliases.py",
            "hook: Hook",
            jump("Hook: by name, 1 match", "depot/crates.py:21"),
        ),
        (
            "aliases.py",
            "hook.seal",
            picker("seal: by name, 7 declarations", &EVERY_SEAL),
        ),
        // `depot` has `Ring` from `depot.loop`, which has it from `depot`.
        (
            "aliases.py",
            "ring: Ring",
            jump("no definition for Ring", "aliases.py:35"),
        ),
    ]);
}

/// #100. `Cls.CONST`, an `Enum` member and a dataclass field are what the class body
/// declares, in the class the qualifier names or one above it: `via Cls`. An attribute a
/// method assigns to `self`, a member of a member, a function's attribute, a class declared
/// inside the function and a parameter of the class's name are not that. A `with … as x`
/// target is a local, which hides the module's name whatever its type (so on master).
#[test]
fn a_python_class_attribute_is_looked_up_in_the_class() {
    let both_delete_user = [
        ("UserRepository.delete_user", "repos.py:8"),
        ("AuditLog.delete_user", "repos.py:13"),
    ];
    py_rows(vec![
        (
            "consts.py",
            "Limits.MAX_USERS",
            jump(
                "MAX_USERS \u{2192} Limits.MAX_USERS (via Limits)",
                "consts.py:12",
            ),
        ),
        (
            "consts.py",
            "Limits.timeout",
            jump(
                "timeout \u{2192} Limits.timeout (via Limits)",
                "consts.py:13",
            ),
        ),
        // Two enums with a `RED`.
        (
            "consts.py",
            "    Color.RED",
            jump("RED \u{2192} Color.RED (via Color)", "consts.py:27"),
        ),
        (
            "consts.py",
            "    Shade.RED",
            jump("RED \u{2192} Shade.RED (via Shade)", "consts.py:32"),
        ),
        // A dataclass field; `Archive.label` is a namesake.
        (
            "consts.py",
            "Point.label",
            jump("label \u{2192} Point.label (via Point)", "consts.py:38"),
        ),
        // Inherited, overridden, and a method as before.
        (
            "consts.py",
            "Tight.MAX_USERS",
            jump(
                "MAX_USERS \u{2192} Limits.MAX_USERS (via Tight)",
                "consts.py:12",
            ),
        ),
        (
            "consts.py",
            "Tight.timeout",
            jump("timeout \u{2192} Tight.timeout (via Tight)", "consts.py:20"),
        ),
        (
            "consts.py",
            "Tight.check",
            jump("check \u{2192} Limits.check (via Tight)", "consts.py:15"),
        ),
        // Behind an import, an alias and a module.
        (
            "consts_use.py",
            "Color.GREEN",
            jump("GREEN \u{2192} Color.GREEN (via Color)", "consts.py:28"),
        ),
        (
            "consts_use.py",
            "Caps.MAX_USERS",
            jump(
                "MAX_USERS \u{2192} Limits.MAX_USERS (via Caps)",
                "consts.py:12",
            ),
        ),
        (
            "consts_use.py",
            "consts.Shade.RED",
            jump("RED \u{2192} Shade.RED (via consts.Shade)", "consts.py:32"),
        ),
        // `self.count = 0` is an instance's.
        (
            "consts.py",
            "Tight.count",
            jump(
                "count \u{2192} Tight.count (by name, 1 match)",
                "consts.py:23",
            ),
        ),
        (
            "consts.py",
            "Limits.missing",
            jump("no definition for missing", "consts.py:48"),
        ),
        (
            "consts.py",
            "Color.RED.value",
            jump(
                "no definition for value (chain broke at Color)",
                "consts.py:53",
            ),
        ),
        (
            "consts.py",
            "scan.cache",
            jump("no definition for cache", "consts.py:54"),
        ),
        // The function's own `class Color`, which the rules do not read (#101).
        (
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
            "consts.py",
            "Color.RED|  # a parameter",
            jump("RED \u{2192} Shade.RED (via Color: Shade)", "consts.py:32"),
        ),
        (
            "consts.py",
            "Limits.MAX_USERS|  # a parameter",
            jump(
                "MAX_USERS \u{2192} Limits.MAX_USERS (by name, 1 match)",
                "consts.py:12",
            ),
        ),
        // The class binds the word in a shape the rules do not read: a tuple, a `def` under
        // an `if`, a `for`. Not reading it is no proof that the base's is meant.
        (
            "consts.py",
            "    Unread.RANK",
            jump(
                "RANK \u{2192} Plain.RANK (by name, 1 match)",
                "consts.py:101",
            ),
        ),
        (
            "consts.py",
            "    Unread.check",
            picker(
                "check: by name, 3 declarations",
                &[
                    ("Limits.check", "consts.py:15"),
                    ("Plain.check", "consts.py:105"),
                    // Under an `if` the walk of `qualified` names nothing.
                    ("check", "consts.py:112"),
                ],
            ),
        ),
        (
            "consts.py",
            "    Unread.CODE",
            jump(
                "CODE \u{2192} Plain.CODE (by name, 1 match)",
                "consts.py:103",
            ),
        ),
        // The body goes on past a comment and a string at column 0, and may share the
        // header's line.
        (
            "consts.py",
            "    Noted.Meta",
            jump("Meta \u{2192} Noted.Meta (via Noted)", "consts.py:133"),
        ),
        (
            "consts.py",
            "    Queried.RANK",
            jump(
                "RANK \u{2192} Plain.RANK (by name, 1 match)",
                "consts.py:101",
            ),
        ),
        (
            "consts.py",
            "    Short.CODE",
            jump(
                "CODE \u{2192} Plain.CODE (by name, 1 match)",
                "consts.py:103",
            ),
        ),
        // The header's own lines at the margin end nothing either.
        (
            "consts.py",
            "    Flush.CODE",
            jump(
                "CODE \u{2192} Plain.CODE (by name, 1 match)",
                "consts.py:103",
            ),
        ),
        // … and a nested class, which is what the class declares.
        (
            "consts.py",
            "    Unread.Meta",
            jump("Meta \u{2192} Unread.Meta (via Unread)", "consts.py:115"),
        ),
        // Two bases that disagree: the order Python reads them in is not computed.
        (
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
        // `with … as`: one line, two targets, and wrapped in brackets.
        (
            "consts.py",
            "        conn|.delete",
            jump("conn \u{2192} scan.conn (local)", "consts.py:70"),
        ),
        (
            "consts.py",
            "        handle|.read",
            jump("handle \u{2192} scan.handle (local)", "consts.py:70"),
        ),
        (
            "consts.py",
            "        wrapped|.delete",
            jump("wrapped: local", "consts.py:74"),
        ),
        (
            "consts.py",
            "conn.delete_user",
            picker("delete_user: by name, 2 declarations", &both_delete_user),
        ),
        (
            "consts.py",
            "wrapped.delete_user",
            picker("delete_user: by name, 2 declarations", &both_delete_user),
        ),
    ]);
}

/// #100. A Python class header wrapped over several lines: its members are the class's,
/// its bases are read, and from inside it a member's implementations are found, the
/// bases of the implementing class sharing one line under theirs.
#[test]
fn a_wrapped_python_class_header_keeps_its_name_and_its_bases() {
    py_rows(vec![
        (
            "impls.py",
            "self.mop",
            jump(
                "mop \u{2192} WrappedJob.mop (via self: WrappedJob)",
                "impls.py:60",
            ),
        ),
        (
            "impls.py",
            "job.mop",
            jump(
                "mop \u{2192} WrappedJob.mop (via job: WrappedJob)",
                "impls.py:60",
            ),
        ),
        (
            "impls.py",
            "def tick",
            jump(
                "tick \u{2192} Narrow.tick (implementations of WideBase.tick)",
                "impls.py:94",
            ),
        ),
        (
            "impls.py",
            "self.sweep",
            jump(
                "sweep \u{2192} Sweeper.sweep (via self: Narrow)",
                "impls.py:44",
            ),
        ),
    ]);
}

/// #100. A parameter of a Python function is named after the function, as a local of its
/// body is: `RecipeController.get_one.slug`, not `RecipeController.slug`, a field's name.
#[test]
fn a_python_parameter_is_named_after_its_function() {
    py_rows(vec![
        (
            "params.py",
            "found = slug",
            jump(
                "slug \u{2192} RecipeController.get_one.slug (local)",
                "params.py:2",
            ),
        ),
        (
            "params.py",
            "return found",
            jump(
                "found \u{2192} RecipeController.get_one.found (local)",
                "params.py:3",
            ),
        ),
        // A signature wrapped over several lines: the parameter, and a local under its `)`.
        (
            "params.py",
            "kept = slugs",
            jump(
                "slugs \u{2192} RecipeController.get_many.slugs (local)",
                "params.py:6",
            ),
        ),
        (
            "params.py",
            "return kept",
            jump(
                "kept \u{2192} RecipeController.get_many.kept (local)",
                "params.py:10",
            ),
        ),
        (
            "params.py",
            "        return slug",
            jump(
                "slug \u{2192} RecipeController.get_later.slug (local)",
                "params.py:13",
            ),
        ),
        (
            "params.py",
            "return level",
            jump("level \u{2192} top.level (local)", "params.py:17"),
        ),
    ]);
}

/// #131. A Python binding that does not start its line is a binding: behind the `:` of a
/// header on the same line, behind a `;`, annotated, chained. It used to be unseen, and the
/// module's `ledger`, an `AuditLog`, was proven in its place. One the rules cannot read hides
/// the module's all the same; a comparison binds nothing.
#[test]
fn a_python_binding_need_not_start_its_line() {
    let users = |n: &str| {
        (
            "scopes.py",
            format!("ledger.delete_user|({n}"),
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via ledger: UserRepository)",
                "repos.py:8",
            ),
        )
    };
    let unproven = |n: &str, status: &str| {
        let rows = [
            ("UserRepository.delete_user", "repos.py:8"),
            ("AuditLog.delete_user", "repos.py:13"),
        ];
        (
            "scopes.py",
            format!("ledger.delete_user|({n}"),
            picker(status, &rows),
        )
    };
    let module = |n: &str| {
        (
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
        // `if fresh: ledger = UserRepository()`.
        users("10"),
        // … and `else: ledger = open("ledger")`.
        unproven("11", by_name),
        // `count = 1; ledger = UserRepository()`.
        users("12 + count"),
        // `for name in names["a:b"]: ledger = …`: the `:` of the string ends no header.
        users("13"),
        // `with … as source: ledger: UserRepository = source`.
        users("14"),
        // The `:` of a header wrapped over two lines, the first ending in `and`, in `(`.
        users("19"),
        users("20"),
        // `first = ledger = UserRepository()`.
        unproven("15 + len", by_name),
        // `try: from fakes import ledger`: only the statement starts with `from`.
        unproven("16", by_name),
        // `if cold: self.ledger = UserRepository()` beside `self.ledger = AuditLog()`.
        unproven(
            "18",
            "delete_user: by name, 2 declarations (chain broke at ledger)",
        ),
        // `if ledger == flag: print(ledger)` binds nothing, and neither does a keyword
        // argument on a line that continues a call: the module's.
        module("17"),
        module("21"),
        // A lambda's `:` behind the end of a call's arguments is no header's.
        module("22"),
    ];
    for (file, code, want) in cases {
        let mut a = fixture_app("python");
        d_on(&mut a, file, &code);
        assert_eq!(shown(&mut a), want, "{file}: {code}");
    }
}

/// Step 5 of #68 over the same project in three languages: a word or a qualifier an import
/// binds to a module of the project is looked for in that module, and the status line names
/// the file or the package directory. A module that does not declare the word re-exports it,
/// and the search by name answers, as before.
#[test]
fn an_import_inside_the_project_is_looked_up_in_its_module() {
    let cases: [(&str, &str, &str, Shown); 13] = [
        // `fakes.py` declares a `UserRepository` too.
        (
            "python",
            "jobs.py",
            "repo: UserRepository",
            jump("UserRepository: via import repos.py", "repos.py:4"),
        ),
        // A package is its `__init__.py`.
        (
            "python",
            "jobs.py",
            "    connect",
            jump(
                "connect: via import store/__init__.py",
                "store/__init__.py:6",
            ),
        ),
        (
            "python",
            "jobs.py",
            "    open_session",
            // `store/__init__.py` imports it from `.sessions` and hands it on (#100).
            jump(
                "open_session: via import store/sessions.py",
                "store/sessions.py:16",
            ),
        ),
        (
            "python",
            "jobs.py",
            "sessions.open_session",
            jump(
                "open_session: via import store/sessions.py",
                "store/sessions.py:16",
            ),
        ),
        // Through an aliased class: its own `start`, not `Pool.start` in the same module.
        (
            "python",
            "jobs.py",
            "StoreSession.start",
            jump(
                "start \u{2192} Session.start (via import store/sessions.py)",
                "store/sessions.py:3",
            ),
        ),
        (
            "python",
            "store/__init__.py",
            "return open_session",
            jump(
                "open_session: via import store/sessions.py",
                "store/sessions.py:16",
            ),
        ),
        (
            "typescript",
            "jobs.ts",
            "repo: UserRepository",
            jump("UserRepository: via import repos.ts", "repos.ts:5"),
        ),
        // A default import under another name: the module's `export default`.
        (
            "typescript",
            "jobs.ts",
            "  connectToStore",
            jump(
                "connectToStore: via import store/index.ts",
                "store/index.ts:5",
            ),
        ),
        (
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
        // A namespace behind the `@/` alias of tsconfig.json.
        (
            "typescript",
            "jobs.ts",
            "sessions.openSession",
            jump(
                "openSession: via import store/sessions.ts",
                "store/sessions.ts:15",
            ),
        ),
        (
            "typescript",
            "jobs.ts",
            "StoreSession.start",
            jump(
                "start \u{2192} Session.start (via import store/sessions.ts)",
                "store/sessions.ts:2",
            ),
        ),
        (
            "typescript",
            "store/index.ts",
            "return openSession",
            jump(
                "openSession: via import store/sessions.ts",
                "store/sessions.ts:15",
            ),
        ),
        // The package function, not the method `Session.Open` nor `fakes.Open`.
        (
            "go",
            "jobs.go",
            "store.Open",
            jump("Open: via import store/", "store/store.go:7"),
        ),
    ];
    for (fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{fixture}: {code}");
    }
}

/// A type that does not declare the member itself hands it to the class it extends or the
/// struct it embeds; a field comes down from a base class the same way. The status line keeps
/// the receiver's own type.
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
    // The project first; a dependency shown relative to its root.
    assert_eq!(
        shown(&mut a),
        picker(
            "find_user: by name, 2 declarations",
            &[
                ("UserRepository.find_user", "repos.py:5"),
                ("Client.find_user", "client/api.py:2"),
            ],
        )
    );
    // TypeScript outside the project is read from its declarations, not the bundle.
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
        )
    );
    std::fs::remove_dir_all(&site).unwrap();
}

/// #333. A word in the module path of an import line opens that module, in the project or outside
/// it, matched from the root it lies under: never a method of the name, never a namesake found
/// by name. So does a name an import binds to a module outside, bare or as a qualifier, unless
/// the package above binds it itself; and the name a plain `import x` binds is a module or
/// nothing.
#[test]
fn a_module_name_in_or_bound_by_an_import_opens_the_module() {
    let std = external_root(
        "py-modules-std",
        &[
            ("json/__init__.py", "def dumps(obj):\n    pass\n"),
            // The base interpreter's pip, which a module found by name used to land in.
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
            // A module of the name deeper in another package is not the one imported.
            ("kombu/utils/yaml.py", "def load(s):\n    pass\n"),
            // A package that binds the name itself keeps its say.
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
    for (code, want) in [
        (
            "from app.repos",
            module("repos", "app/repos.py", Path::new("")),
        ),
        (
            "from app|.repos",
            module("app", "app/__init__.py", Path::new("")),
        ),
        (
            "from django.core.management",
            module("management", "django/core/management/__init__.py", &site),
        ),
        (
            "from django.core|.management",
            module("core", "django/core/__init__.py", &site),
        ),
        ("^import json", module("json", "json/__init__.py", &std)),
        ("json|.dumps", module("json", "json/__init__.py", &std)),
        (
            "import serializers",
            module("serializers", "rest_framework/serializers.py", &site),
        ),
        (
            "serializers|.ListField",
            module("serializers", "rest_framework/serializers.py", &site),
        ),
        // Nothing of the name at the root: no module, and no search by name.
        (
            "^import yaml",
            jump("no definition for yaml", "app/modules.py:5"),
        ),
        (
            "^    yaml|.load",
            jump("no definition for yaml", "app/modules.py:12"),
        ),
        // Unchanged: what the import takes, and a name the package above binds.
        (
            "import UserRepo",
            jump("UserRepo: via import app/repos.py", "app/repos.py:1"),
        ),
        (
            "json.dumps",
            jump(
                "dumps: via import json",
                &format!("{}:1", std.join("json/__init__.py").display()),
            ),
        ),
        (
            "serializers.ListField",
            jump(
                "ListField: via import rest_framework.serializers",
                &format!("{}:1", site.join("rest_framework/serializers.py").display()),
            ),
        ),
        (
            "import fields",
            jump(
                "fields: by name, 1 match",
                &format!("{}:1", site.join("drf/__init__.py").display()),
            ),
        ),
    ] {
        d_on(&mut a, "app/modules.py", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    for d in [dir, std, site] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

/// #342. A member a project class lacks, when its ancestry goes outside the project, is looked
/// for only in the project classes extending it; a name qualified by a class imported from
/// outside is a member of that class, never a top-level namesake; one method outside is offered,
/// not jumped to, while a field of the name is declared outside too.
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
            // Django's `settings` is an instance whose `__getattr__` reads the project's
            // settings module (#560): its members are nowhere in `django/conf`.
            (
                "django/conf/__init__.py",
                "class LazySettings:\n    def __getattr__(self, name):\n        pass\n\n\nsettings = LazySettings()\n",
            ),
            // Namesakes outside, in the module's package and elsewhere, are no setting of the
            // project.
            ("django/conf/global_settings.py", "ORIGINALS_DIR = None\n"),
            ("otherlib/consts.py", "ORIGINALS_DIR = 1\n"),
            // A class declared under an `if` is a class of the module all the same.
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
                "ORIGINALS_DIR = \"originals\"\nclient = None\n",
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
                "from django.conf import settings\n\n\ndef originals():\n    settings.connect()\n    return settings.ORIGINALS_DIR\n",
            ),
            // A subclass that sets the member stays a candidate.
            (
                "app/test_api.py",
                "from django.test import TestCase\n\n\nclass ApiCase(TestCase):\n    def test_post(self) -> None:\n        self.client.post(\"/\")\n\n\nclass SignedCase(ApiCase):\n    def setUp(self) -> None:\n        self.client = None\n",
            ),
            // A base written as a call cannot be read: today's search by name.
            (
                "app/test_six.py",
                "import six\nfrom django.test import TestCase\n\n\nclass SixCase(six.with_metaclass(type, TestCase)):\n    def test_put(self) -> None:\n        self.client.put(\"/\")\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Python, &[std.clone(), site.clone()]);
    let outside = |root: &Path, file: &str| format!("{}", root.join(file).display());
    for (file, code, want) in [
        (
            "app/test_views.py",
            "self.client",
            jump("no definition for client", "app/test_views.py:6"),
        ),
        (
            "app/users.py",
            "User.objects",
            jump("no definition for objects", "app/users.py:5"),
        ),
        // Not a class of the module: what it holds is not read, the search by name answers
        // (#560).
        (
            "app/test_files.py",
            "settings.ORIGINALS_DIR",
            jump("ORIGINALS_DIR: by name, 1 match", "app/settings.py:1"),
        ),
        // Only a module-level assignment of the project: never a method of a project class.
        (
            "app/test_files.py",
            "settings.connect",
            jump("no definition for connect", "app/test_files.py:5"),
        ),
        (
            "app/things.py",
            "Thing.objects",
            jump(
                "objects \u{2192} Thing.objects (via import condpkg.models)",
                &format!("{}:3", outside(&site, "condpkg/models.py")),
            ),
        ),
        // A class the module re-exports by an import is read no further than that module.
        (
            "app/things.py",
            "TestCase.client",
            jump("no definition for client", "app/things.py:6"),
        ),
        (
            "app/mocks.py",
            "m.return_value",
            Shown::Picker(
                "return_value: by name, 1+ declarations".into(),
                vec![(
                    "TaskHandle.return_value".into(),
                    "by name".into(),
                    "anyio/tasks.py:2".into(),
                )],
            ),
        ),
        (
            "app/mocks.py",
            "m.captured_queries",
            jump(
                "captured_queries \u{2192} TaskHandle.captured_queries (by name, 1 match)",
                &format!("{}:5", outside(&site, "anyio/tasks.py")),
            ),
        ),
        (
            "app/test_api.py",
            "self.client|.post",
            jump(
                "client \u{2192} SignedCase.client (by name, 1 match)",
                "app/test_api.py:11",
            ),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{file}: {code}");
    }
    d_on(&mut a, "app/test_six.py", "self.client");
    let Shown::Picker(_, rows) = shown(&mut a) else {
        panic!("a picker");
    };
    assert!(rows.iter().any(|r| r.2 == "app/mailer.py:3"), "{rows:?}");
    for d in [dir, std, site] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

/// #336. A builtin has no source: a bare `next` nothing in the file binds, and a member of a
/// value proven to be a `str`, say so rather than jumping to a namesake outside. A bare name
/// nothing binds that is no builtin is no method outside, and only a module the file
/// `*`-imports can declare it there.
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
            // A project class called `str` is read as before.
            (
                "app/own_str.py",
                "class str:\n    def replace(self):\n        pass\n\n\ndef go(s: str) -> None:\n    s.replace()\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&std));
    let here = |file: &str, line: usize| format!("{file}:{line}");
    for (file, code, want) in [
        (
            "app/builtins_use.py",
            "s.replace",
            jump(
                "replace: builtin, no source (via render() -> str)",
                &here("app/builtins_use.py", 7),
            ),
        ),
        (
            "app/builtins_use.py",
            "next",
            jump("next: builtin, no source", &here("app/builtins_use.py", 8)),
        ),
        (
            "app/builtins_use.py",
            "name.upper",
            jump(
                "upper: builtin, no source (via name: str)",
                &here("app/builtins_use.py", 9),
            ),
        ),
        (
            "app/builtins_use.py",
            "^    helper",
            jump("no definition for helper", &here("app/builtins_use.py", 10)),
        ),
        (
            "app/starred.py",
            "^    helper",
            jump(
                "helper: via import tools",
                &format!("{}:1", std.join("tools/__init__.py").display()),
            ),
        ),
        (
            "app/own_str.py",
            "s.replace",
            jump(
                "replace \u{2192} str.replace (via s: str)",
                &here("app/own_str.py", 2),
            ),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{file}: {code}");
    }
    std::fs::remove_dir_all(dir).unwrap();
    std::fs::remove_dir_all(std).unwrap();
}
