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
