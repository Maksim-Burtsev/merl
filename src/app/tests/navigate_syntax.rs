//! `d` over wrapped headers, broken chains and the shapes of a name.

use super::*;

/// #100, TypeScript: a class header prettier wraps is still the class's header. A list of type
/// parameters over several lines ends in `> extends Base<K> {`, the clauses may stand on lines
/// of their own over a lone `{`; `this`, `super`, the fields and what the class extends are
/// read through both. The parameters of a function behind a wrapped `<…>` hide a module's
/// namesake.
#[test]
fn a_wrapped_class_header_is_a_header() {
    let user = |via: &str| {
        jump(
            &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
            "repos.ts:10",
        )
    };
    let audit = |via: &str| {
        jump(
            &format!("deleteUser \u{2192} AuditLog.deleteUser (via {via})"),
            "repos.ts:16",
        )
    };
    let seal = |via: &str| {
        jump(
            &format!("seal \u{2192} Crate.seal (via {via})"),
            "headers.ts:12",
        )
    };
    let cases: Vec<(&str, Shown)> = vec![
        // Under `> extends Crate<K> {`: a field of the class, one of its base, a method of
        // the base through `this` and through `super`, a constructor parameter.
        (
            "this.repo.deleteUser|(id)",
            user("this.repo: UserRepository"),
        ),
        (
            "this.audit.deleteUser|(id + 1)",
            audit("this.audit: AuditLog"),
        ),
        ("this.seal|(key)", seal("this: Shelf")),
        ("super.seal|(key)", seal("super of Shelf")),
        (
            "this.spare|)",
            jump(
                "spare \u{2192} Shelf.spare (via this: Shelf)",
                "headers.ts:34",
            ),
        ),
        // Under `extends` and `implements` on their own lines and a lone `{`.
        (
            "this.repo.deleteUser|(id + 2)",
            user("this.repo: UserRepository"),
        ),
        (
            "this.audit.deleteUser|(id + 3)",
            audit("this.audit: AuditLog"),
        ),
        (
            "super.seal|(key + \"!\")",
            seal("super of LongNamedShelfOfStrings"),
        ),
        // `implements Sealable` on its own line is read; the constraint `S extends Sealable`
        // of `Bin` implements nothing.
        (
            "seal|(key: string): void;",
            jump(
                "seal \u{2192} LongNamedShelfOfStrings.seal (implementations of Sealable.seal)",
                "headers.ts:70",
            ),
        ),
        // A method whose own type parameters are wrapped, `stash<` over `>(a: A, b: B)`; a
        // call written so inside a method is no declaration of it.
        (
            "this.stash|(key, this.spare)",
            jump(
                "stash \u{2192} Crate.stash (via this: Shelf)",
                "headers.ts:18",
            ),
        ),
        // What overrides a method of the base, and a field found by name, are told to be
        // the class's under `> extends … {` too.
        (
            "^  open|(): void {}",
            jump(
                "open \u{2192} Shelf.open (implementations of Crate.open)",
                "headers.ts:51",
            ),
        ),
        (
            "found.spare|)",
            jump(
                "spare \u{2192} Shelf.spare (by name, 1 match)",
                "headers.ts:34",
            ),
        ),
        (
            "found.one|)",
            jump("one \u{2192} Bin.one (by name, 1 match)", "headers.ts:81"),
        ),
        // `other: T` is typed by a parameter of the wrapped list: the chain breaks there.
        (
            "this.other.seal|(key)",
            picker(
                "seal: by name, 4 declarations (chain broke at other)",
                &[
                    ("Sealable.seal", "headers.ts:6"),
                    ("Crate.seal", "headers.ts:12"),
                    ("LongNamedShelfOfStrings.seal", "headers.ts:70"),
                    ("Bin.seal", "headers.ts:85"),
                ],
            ),
        ),
        // `type Loose = any;` has no body: the fields of the class under it are not its.
        (
            "loose.audit.deleteUser|(id + 5)",
            picker(
                "deleteUser: by name, 2 declarations (chain broke at audit)",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        // A lone `{` under a statement, with no `;`, is a block, and the statement still binds.
        (
            "void repo.deleteUser|(id + 4)",
            user("repo: UserRepository"),
        ),
        // `>(repo: UserRepository, …` binds the parameter: the module's `repo` is hidden.
        (
            "void repo.deleteUser|(key.length)",
            user("repo: UserRepository"),
        ),
        ("^  repo.deleteUser|(id)", audit("repo: AuditLog")),
        // `(repo: AuditLog) => void` among wrapped type parameters, or wrapped type
        // arguments, types the function's `repo` no more than it does on one line.
        (
            "void repo.deleteUser|(visit.length);",
            user("repo: UserRepository"),
        ),
        (
            "void repo.deleteUser|(visit.length + 1)",
            user("repo: UserRepository"),
        ),
    ];
    for (code, want) in cases {
        let mut a = fixture_app("typescript");
        d_on(&mut a, "headers.ts", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
}

/// #100, TypeScript: a member access prettier broke in front of its dots reads as the one line
/// it is; a comment at the end of the line above names no receiver.
#[test]
fn a_member_access_broken_over_lines_is_one_chain() {
    let user = |via: &str| {
        jump(
            &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
            "repos.ts:10",
        )
    };
    let by_name = || {
        picker(
            "deleteUser: by name, 2 declarations",
            &[
                ("UserRepository.deleteUser", "repos.ts:10"),
                ("AuditLog.deleteUser", "repos.ts:16"),
            ],
        )
    };
    let cases: Vec<(&str, Shown)> = vec![
        // One break, two breaks with a comment between them, and the name in the middle.
        (
            "      .deleteUser|(id);",
            user("this.uow: UnitOfWork \u{2192} users: UserRepository"),
        ),
        (
            "      .deleteUser|(id + 1);",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via this.uow: UnitOfWork \u{2192} audit: AuditLog)",
                "repos.ts:16",
            ),
        ),
        (
            "      .audit",
            jump(
                "audit \u{2192} UnitOfWork.audit (via this.uow: UnitOfWork)",
                "chains.ts:5",
            ),
        ),
        // Off a method's call and off a function's, as on one line.
        (
            "      .deleteUser|(id + 2);",
            user("this.depot.peopleRepo(): UserRepository"),
        ),
        (
            "      .peopleRepo|()",
            jump(
                "peopleRepo \u{2192} Depot.peopleRepo (via this.depot: Depot)",
                "calls.ts:10",
            ),
        ),
        (
            "      .people.deleteUser|(id + 3);",
            user("openDepot(): Depot \u{2192} people: UserRepository"),
        ),
        (
            "      .people|.deleteUser(id + 3);",
            jump(
                "people \u{2192} Depot.people (via openDepot(): Depot)",
                "calls.ts:8",
            ),
        ),
        // `found // note`: the module's `note` is an `AuditLog`, and no receiver here.
        ("      .deleteUser|(id + 4);", by_name()),
        // A call of a call, and a call closed on a line of its own, stay by name.
        ("      .deleteUser|(id + 5);", by_name()),
        ("      .deleteUser|(id + 6);", by_name()),
        (
            "      .length",
            jump("no definition for length", "fluent.ts:39"),
        ),
    ];
    for (code, want) in cases {
        let mut a = fixture_app("typescript");
        d_on(&mut a, "fluent.ts", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
}

/// #100, TypeScript: `r!.m()` and `a?.b.m()` have the type of the plain access for a member
/// lookup.
#[test]
fn a_non_null_or_optional_access_is_the_plain_one() {
    let user = |via: &str| {
        jump(
            &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
            "repos.ts:10",
        )
    };
    let audit = |via: &str| {
        jump(
            &format!("deleteUser \u{2192} AuditLog.deleteUser (via {via})"),
            "repos.ts:16",
        )
    };
    let users = "this.uow: UnitOfWork \u{2192} users: UserRepository";
    let cases: Vec<(&str, Shown)> = vec![
        (
            "this.repo!.deleteUser|(id)",
            user("this.repo: UserRepository"),
        ),
        (
            "this.repo?.deleteUser|(id + 1)",
            user("this.repo: UserRepository"),
        ),
        ("this.uow?.users.deleteUser|(id + 2)", user(users)),
        (
            "this.uow?.users|.deleteUser(id + 2)",
            jump(
                "users \u{2192} UnitOfWork.users (via this.uow: UnitOfWork)",
                "chains.ts:4",
            ),
        ),
        // Two marks in one chain, and two around a name of one letter.
        (
            "this.uow!.audit!.deleteUser|(id + 3)",
            audit("this.uow: UnitOfWork \u{2192} audit: AuditLog"),
        ),
        (
            "u!.users!.deleteUser|(id + 9)",
            user("u.users: UserRepository"),
        ),
        ("spare?.deleteUser|(id + 4)", audit("spare: AuditLog")),
        ("spare!.deleteUser|(id + 5)", audit("spare: AuditLog")),
        // A receiver nobody typed stays by name.
        (
            "found?.deleteUser|(id + 6)",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        ("!note.deleteUser|.length", audit("note: AuditLog")),
    ];
    for (code, want) in cases {
        let mut a = fixture_app("typescript");
        d_on(&mut a, "optional.ts", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
}

/// #100, TypeScript: a `#private` member is the word with its `#`, on the `#` and on the name.
#[test]
fn a_private_name_keeps_its_hash() {
    let private = jump(
        "#addRoute \u{2192} Router.#addRoute (via this: Router)",
        "privates.ts:12",
    );
    let cases: Vec<(&str, Shown)> = vec![
        // On the name and on the `#`: the private method, not the public `addRoute`.
        ("this.#addRoute|(path);", private),
        (
            "this.|#addRoute(path + \"/\")",
            jump(
                "#addRoute \u{2192} Router.#addRoute (via this: Router)",
                "privates.ts:12",
            ),
        ),
        // A private field, as a target and as a link.
        (
            "this.#repo|.deleteUser(id)",
            jump(
                "#repo \u{2192} Router.#repo (via this: Router)",
                "privates.ts:5",
            ),
        ),
        (
            "console.log(this.#audit|)",
            jump(
                "#audit \u{2192} Router.#audit (via this: Router)",
                "privates.ts:6",
            ),
        ),
        (
            "other.#repo.deleteUser|(id + 2)",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via other.#repo: UserRepository)",
                "repos.ts:10",
            ),
        ),
        // The public name never reaches a private one, by name or through a type.
        (
            "found.addRoute|(path)",
            jump(
                "addRoute \u{2192} Router.addRoute (by name, 1 match)",
                "privates.ts:16",
            ),
        ),
        (
            "this.addRoute|(path);",
            jump(
                "addRoute \u{2192} Router.addRoute (via this: SubRouter)",
                "privates.ts:16",
            ),
        ),
        // `route#addRoute` in a string is no private name.
        (
            "see route#addRoute|",
            jump(
                "addRoute \u{2192} Router.addRoute (by name, 1 match)",
                "privates.ts:16",
            ),
        ),
        // A subclass's `#addRoute` is its own, and implements nothing of the base's.
        (
            "this.#addRoute|(path, 1)",
            jump(
                "#addRoute \u{2192} SubRouter.#addRoute (via this: SubRouter)",
                "privates.ts:37",
            ),
        ),
        (
            "^  #addRoute|(path: string): void {",
            picker(
                "#addRoute: at a declaration, 1 other by name",
                &[("SubRouter.#addRoute", "privates.ts:37")],
            ),
        ),
    ];
    for (code, want) in cases {
        let mut a = fixture_app("typescript");
        d_on(&mut a, "privates.ts", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
}

/// #100, TypeScript: a NestJS service. Its dependencies are constructor parameters wrapped one
/// to a line, decorated or not; `const { repo } = this` hands fields on; a class may stand
/// behind namespaces, of an import or of the file itself.
#[test]
fn a_nest_service_reads_its_dependencies() {
    let user = |via: &str| {
        jump(
            &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
            "repos.ts:10",
        )
    };
    let audit = |via: &str| {
        jump(
            &format!("deleteUser \u{2192} AuditLog.deleteUser (via {via})"),
            "repos.ts:16",
        )
    };
    let field = |name: &str, line: usize| {
        jump(
            &format!("{name} \u{2192} AlbumService.{name} (via this: AlbumService)"),
            &format!("nest.ts:{line}"),
        )
    };
    let spin = |via: &str, to: &str, line: usize| {
        jump(
            &format!("spin \u{2192} {to}.spin (via {via})"),
            &format!("nest_parts.ts:{line}"),
        )
    };
    let cases: Vec<(&str, Shown)> = vec![
        // The wrapped constructor: a decorated parameter, a plain one, one under its
        // decorator's line; as a link and as a target.
        (
            "this.repo.deleteUser|(id)",
            user("this.repo: UserRepository"),
        ),
        (
            "this.audit.deleteUser|(id + 1)",
            audit("this.audit: AuditLog"),
        ),
        (
            "this.uow.users.deleteUser|(id + 2)",
            user("this.uow: UnitOfWork \u{2192} users: UserRepository"),
        ),
        ("console.log(this.repo|,", field("repo", 15)),
        ("console.log(this.repo, this.audit|,", field("audit", 16)),
        (
            "console.log(this.repo, this.audit, this.uow|)",
            field("uow", 18),
        ),
        // `const { repo, audit: trail } = this` and `const { users } = this.uow`; the
        // module's `repo` is an `AuditLog`.
        (
            "void repo.deleteUser|(id + 3)",
            user("repo: UserRepository"),
        ),
        ("trail.deleteUser|(id + 4)", audit("trail: AuditLog")),
        (
            "void users.deleteUser|(id + 5)",
            user("users: UserRepository"),
        ),
        // A default may be what the name holds.
        (
            "uow.audit.deleteUser|(id + 6)",
            picker(
                "deleteUser: by name, 2 declarations (chain broke at uow)",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        ("^  repo.deleteUser|(id + 7)", audit("repo: AuditLog")),
        // Namespaces of an import, of a module taken whole, and of the file itself, where a
        // `Tool` and a `Widget` outside them are other classes.
        (
            "widget.spin|(id)",
            spin("widget: Widget", "Outer.Inner.Widget", 4),
        ),
        (
            "new Outer.Inner.Widget().spin|(id + 1)",
            spin("new Outer.Inner.Widget(): Widget", "Outer.Inner.Widget", 4),
        ),
        (
            "other.spin|(id + 2)",
            spin("other: Widget", "Outer.Inner.Widget", 4),
        ),
        (
            "gadget.spin|(id + 3)",
            spin("gadget: Gadget", "Outer.Gadget", 11),
        ),
        (
            "tool.turn|(id)",
            jump(
                "turn \u{2192} Local.Tool.turn (via tool: Tool)",
                "nest.ts:46",
            ),
        ),
        (
            "new Local.Tool().turn|(id + 1)",
            jump(
                "turn \u{2192} Local.Tool.turn (via new Local.Tool(): Tool)",
                "nest.ts:46",
            ),
        ),
        (
            "new Tool().turn|(id + 2)",
            jump(
                "turn \u{2192} Tool.turn (via new Tool(): Tool)",
                "nest.ts:53",
            ),
        ),
    ];
    for (code, want) in cases {
        let mut a = fixture_app("typescript");
        d_on(&mut a, "nest.ts", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
}

/// #131, TypeScript: a destructuring prettier wrapped over several lines binds its names, so
/// the module's `ledger`, an `AuditLog`, does not answer for them.
#[test]
fn a_wrapped_destructuring_binds_its_names() {
    let user = jump(
        "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
        "repos.ts:10",
    );
    let by_name = || {
        picker(
            "deleteUser: by name, 2 declarations",
            &[
                ("UserRepository.deleteUser", "repos.ts:10"),
                ("AuditLog.deleteUser", "repos.ts:16"),
            ],
        )
    };
    let cases: Vec<(&str, Shown)> = vec![
        // Out of `deps: Deps`, whose `ledger` is a `UserRepository`; from a block below too.
        ("ledger.deleteUser|(id + 13)", user),
        (
            "ledger.deleteUser|(id + 14)",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                "repos.ts:10",
            ),
        ),
        // With a type literal behind the pattern, and an array's pattern: nothing is read,
        // and nothing outside answers.
        ("ledger.deleteUser|(count + 15)", by_name()),
        ("ledger.deleteUser|(16)", by_name()),
        // Any statement closed so is read whole, once: `const ledger = {` … `} as T;`.
        (
            "ledger.deleteUser|(id + 17)",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                "repos.ts:10",
            ),
        ),
    ];
    for (code, want) in cases {
        let mut a = fixture_app("typescript");
        d_on(&mut a, "scopes.ts", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
}

/// #100, TypeScript workspaces: a file sees the `node_modules` of every directory above it, the
/// nearest first, and not those of the package beside it.
#[test]
fn a_workspace_package_sees_the_node_modules_above_it() {
    let main = "import { pick } from \"lib\";\n\npick(1);\n";
    let (dir, mut a) = project_app(
        "workspace",
        &[
            ("packages/api/src/main.ts", main),
            ("packages/web/src/main.ts", main),
            ("main.ts", main),
        ],
    );
    // Written after the project walk, which a `.gitignore` keeps out of them.
    for (path, text) in [
        (
            "packages/api/node_modules/lib/index.d.ts",
            "import { deep } from \"deep\";\nexport declare function pick(n: number): number;\nexport declare const made: typeof deep;\n",
        ),
        (
            "packages/api/node_modules/lib/node_modules/deep/index.d.ts",
            "export declare function deep(): void;\n",
        ),
        (
            "node_modules/lib/index.d.ts",
            "// An older one.\nexport declare function pick(n: string): string;\n",
        ),
    ] {
        std::fs::create_dir_all(dir.join(path).parent().unwrap()).unwrap();
        std::fs::write(dir.join(path), text).unwrap();
    }
    let top = || jump("pick: via import lib", "node_modules/lib/index.d.ts:2");
    let api = || {
        jump(
            "pick: via import lib",
            "packages/api/node_modules/lib/index.d.ts:2",
        )
    };
    // Back in `api` after `web`: each file has its own view, and no directory is walked twice.
    for (file, want) in [
        ("packages/api/src/main.ts", api()),
        ("packages/web/src/main.ts", top()),
        ("main.ts", top()),
        ("packages/api/src/main.ts", api()),
    ] {
        d_on(&mut a, file, "^pick");
        assert_eq!(shown(&mut a), want, "{file}");
    }
    assert_eq!(a.node_modules.len(), 2);
    // From inside a dependency, its own `node_modules` is the nearest, and the one it lies
    // in is not listed twice.
    d_on(
        &mut a,
        "packages/api/node_modules/lib/index.d.ts",
        "made: typeof deep",
    );
    assert_eq!(
        shown(&mut a),
        jump(
            "deep: via import deep",
            "packages/api/node_modules/lib/node_modules/deep/index.d.ts:1"
        )
    );
    assert_eq!(a.buf.readonly, Some("outside the project"));
    // A dependency of `api` is outside the project from wherever `d` was pressed last.
    d_on(&mut a, "packages/web/src/main.ts", "^pick");
    a.jump_to(&dir.join("packages/api/node_modules/lib/index.d.ts"), 1);
    assert_eq!(a.buf.readonly, Some("outside the project"));
    d_on(
        &mut a,
        "packages/api/node_modules/lib/index.d.ts",
        "made: typeof deep",
    );
    let (roots, files) = a.external[&Kind::TsJs].clone();
    assert_eq!(
        roots,
        [
            dir.join("packages/api/node_modules/lib/node_modules"),
            dir.join("packages/api/node_modules"),
            dir.join("node_modules"),
        ]
    );
    assert_eq!(files.len(), 3);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #141: a package installed more than once is the copy Node and TypeScript load, the one in
/// the nearest `node_modules` that has it or its `@types`, and not a copy another package
/// depends on. A name only another copy declares is found by name.
#[test]
fn an_imported_package_is_the_copy_node_loads() {
    let main = "import { pick, onlyFar } from \"lib\";\nimport { part } from \"lib/sub\";\nimport { nest } from \"nested\";\nimport { typed } from \"typed\";\nimport { scoped } from \"@scope/pkg\";\nimport { readFile } from \"fs\";\nimport { Buffer } from \"buffer\";\n\npick(1);\nonlyFar(1);\npart(1);\nnest(1);\ntyped(1);\nscoped(1);\nreadFile(1);\nBuffer.from(1);\n";
    let file = "packages/api/src/main.ts";
    let (dir, mut a) = project_app("copies", &[(file, main)]);
    for (path, names) in [
        ("packages/api/node_modules/lib/index.d.ts", &["pick"][..]),
        ("node_modules/lib/index.d.ts", &["pick", "onlyFar"]),
        ("packages/api/node_modules/lib/sub.d.ts", &["part"]),
        ("node_modules/lib/sub.d.ts", &["part"]),
        // A copy another package depends on, and a package the copy itself depends on.
        ("node_modules/nested/index.d.ts", &["nest"]),
        (
            "node_modules/other/node_modules/nested/index.d.ts",
            &["nest"],
        ),
        ("node_modules/nested/node_modules/dep/index.d.ts", &["nest"]),
        // Types with no package beside them, a scoped package's among them.
        (
            "packages/api/node_modules/@types/typed/index.d.ts",
            &["typed"],
        ),
        ("node_modules/typed/index.d.ts", &["typed"]),
        (
            "packages/api/node_modules/@types/scope__pkg/index.d.ts",
            &["scoped"],
        ),
        ("node_modules/@scope/pkg/index.d.ts", &["scoped"]),
        // No package is called `fs`: it is Node's own, typed by `@types/node`.
        ("node_modules/@types/node/fs.d.ts", &["readFile"]),
    ] {
        let text: String = names
            .iter()
            .map(|n| format!("export declare function {n}(): void;\n"))
            .collect();
        std::fs::create_dir_all(dir.join(path).parent().unwrap()).unwrap();
        std::fs::write(dir.join(path), text).unwrap();
    }
    // `buffer` is Node's own too, whatever npm polyfill of that name is installed.
    std::fs::create_dir_all(dir.join("node_modules/buffer")).unwrap();
    for (path, text) in [
        (
            "node_modules/buffer/index.d.ts",
            "export declare class Buffer {\n}\n",
        ),
        (
            "node_modules/@types/node/buffer.d.ts",
            "declare module \"buffer\" {\n    export class Buffer {\n    }\n}\n",
        ),
    ] {
        std::fs::write(dir.join(path), text).unwrap();
    }
    for (code, want) in [
        (
            "^pick",
            jump(
                "pick: via import lib",
                "packages/api/node_modules/lib/index.d.ts:1",
            ),
        ),
        (
            "^onlyFar",
            jump("onlyFar: by name, 1 match", "node_modules/lib/index.d.ts:2"),
        ),
        (
            "^part",
            jump(
                "part: via import lib/sub",
                "packages/api/node_modules/lib/sub.d.ts:1",
            ),
        ),
        (
            "^nest",
            jump(
                "nest: via import nested",
                "node_modules/nested/index.d.ts:1",
            ),
        ),
        (
            "^typed",
            jump(
                "typed: via import typed",
                "packages/api/node_modules/@types/typed/index.d.ts:1",
            ),
        ),
        (
            "^scoped",
            jump(
                "scoped: via import @scope/pkg",
                "packages/api/node_modules/@types/scope__pkg/index.d.ts:1",
            ),
        ),
        (
            "^readFile",
            jump(
                "readFile: via import fs",
                "node_modules/@types/node/fs.d.ts:1",
            ),
        ),
        (
            "^Buffer",
            Shown::Picker(
                "Buffer: via import buffer, 2 declarations".into(),
                ["@types/node/buffer.d.ts:2", "buffer/index.d.ts:1"]
                    .map(|at| ("Buffer".into(), "via import buffer".into(), at.into()))
                    .to_vec(),
            ),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #141, pnpm: `node_modules/lib` links the version of the store a file loads. A workspace
/// package linked in has no file in the walk, and is looked for as before.
#[cfg(unix)]
#[test]
fn a_linked_package_is_the_version_it_links() {
    let main = "import { pin } from \"pinned\";\nimport { shared } from \"@app/shared\";\n\npin(1);\nshared(1);\n";
    let (dir, mut a) = project_app(
        "linked",
        &[
            ("src/main.ts", main),
            ("packages/shared/index.ts", "export function shared() {}\n"),
        ],
    );
    for version in ["1.0.0", "2.0.0"] {
        let path = dir.join(format!(
            "node_modules/.pnpm/pinned@{version}/node_modules/pinned/index.d.ts"
        ));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "export declare function pin(): void;\n").unwrap();
    }
    std::fs::create_dir_all(dir.join("node_modules/@app")).unwrap();
    for (to, at) in [
        (
            ".pnpm/pinned@2.0.0/node_modules/pinned",
            "node_modules/pinned",
        ),
        ("../../packages/shared", "node_modules/@app/shared"),
    ] {
        std::os::unix::fs::symlink(to, dir.join(at)).unwrap();
    }
    for (code, want) in [
        (
            "^pin",
            jump(
                "pin: via import pinned",
                "node_modules/.pnpm/pinned@2.0.0/node_modules/pinned/index.d.ts:1",
            ),
        ),
        (
            "^shared",
            jump("shared: by name, 1 match", "packages/shared/index.ts:1"),
        ),
    ] {
        d_on(&mut a, "src/main.ts", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #141 is npm's rule. Python puts a namespace package together from every root that has a
/// part of it, so `google.cloud` is found behind a `google` in the first root.
#[test]
fn a_python_namespace_package_spans_its_roots() {
    let (dir, mut a) = project_app(
        "namespace",
        &[(
            "m.py",
            "from google.cloud import storage\n\nstorage.Client()\n",
        )],
    );
    let first = external_root(
        "ns-first",
        &[("google/protobuf/message.py", "class Message:\n    pass\n")],
    );
    let second = external_root(
        "ns-second",
        &[(
            "google/cloud/storage/__init__.py",
            "class Client:\n    pass\n",
        )],
    );
    use_roots(&mut a, Kind::Python, &[first.clone(), second.clone()]);
    d_on(&mut a, "m.py", "storage.Client");
    let at = second.join("google/cloud/storage/__init__.py");
    assert_eq!(
        shown(&mut a),
        jump(
            "Client: via import google.cloud.storage",
            &format!("{}:1", at.display())
        )
    );
    for d in [dir, first, second] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

/// #100, TypeScript: `export { Trunk as TrunkBase }` is followed to the class the module
/// declares under its own name, as hono exports the `HonoBase` its `Hono` extends. A re-export
/// from another module under a new name is not.
#[test]
fn an_export_under_another_name_is_followed() {
    let cases: Vec<(&str, Shown)> = vec![
        (
            "super.lock|()",
            jump(
                "lock \u{2192} Trunk.lock (via super of Boot)",
                "aliased.ts:7",
            ),
        ),
        (
            "this.audit.deleteUser|(1)",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via this.audit: AuditLog)",
                "repos.ts:16",
            ),
        ),
        (
            "trunk.lock|()",
            jump(
                "lock \u{2192} Trunk.lock (via trunk: Trunk)",
                "aliased.ts:7",
            ),
        ),
        (
            "extends TrunkBase|",
            jump("TrunkBase: via import aliased.ts", "aliased.ts:4"),
        ),
        (
            "import { HatchBase|",
            jump("no definition for HatchBase", "aliased_use.ts:1"),
        ),
        // `HatchBase` is the `UserRepository` of `repos`, not the one `aliased` declares.
        (
            "hatch.deleteUser|(2)",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
    ];
    for (code, want) in cases {
        let mut a = fixture_app("typescript");
        d_on(&mut a, "aliased_use.ts", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
}

/// What the Punchcard review of the TypeScript items (#100, #131) found: each row was a wrong
/// jump, a lost one or a picker of doubles on the first build of them.
#[test]
fn what_the_review_of_the_typescript_items_found() {
    let user = |via: &str| {
        jump(
            &format!("deleteUser \u{2192} UserRepository.deleteUser (via {via})"),
            "repos.ts:10",
        )
    };
    let by_name = || {
        picker(
            "deleteUser: by name, 2 declarations",
            &[
                ("UserRepository.deleteUser", "repos.ts:10"),
                ("AuditLog.deleteUser", "repos.ts:16"),
            ],
        )
    };
    let public = || {
        jump(
            "addRoute \u{2192} Router.addRoute (by name, 1 match)",
            "privates.ts:16",
        )
    };
    let cases: Vec<(&str, &str, Shown)> = vec![
        // `svc.list()` is the namespace's function, until a parameter `svc` hides it: as a
        // callee and as the namespace of `new svc.Tool()`.
        (
            "shadowed.ts",
            "log.deleteUser|(id)",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via svc.list(): AuditLog[])",
                "repos.ts:16",
            ),
        ),
        ("shadowed.ts", "r.deleteUser|(id + 1)", by_name()),
        (
            "shadowed.ts",
            "tool.turn|(String(id + 2))",
            picker(
                "turn: by name, 3 declarations",
                &[
                    ("svc.Tool.turn", "shadowed.ts:10"),
                    ("Local.Tool.turn", "nest.ts:46"),
                    ("Tool.turn", "nest.ts:53"),
                ],
            ),
        ),
        // A statement closed by `}` is one declaration, not its first line and itself.
        (
            "scopes.ts",
            "void ledger|.deleteUser(id + 17)",
            jump(
                "ledger \u{2192} wrappedCast.ledger (local)",
                "scopes.ts:129",
            ),
        ),
        // `this.stash<` over its type arguments over `>(key, this.spare);` is a call.
        (
            "headers.ts",
            "found.stash|(1, {})",
            jump(
                "stash \u{2192} Crate.stash (by name, 1 match)",
                "headers.ts:18",
            ),
        ),
        // #131 under a name commented out at the margin, and behind another name's default.
        (
            "scopes.ts",
            "ledger.deleteUser|(18)",
            user("ledger: UserRepository"),
        ),
        ("scopes.ts", "ledger?.deleteUser|(id + 19)", by_name()),
        // A `#` in a comment or a string starts no private name.
        ("privates.ts", "// As #addRoute|", public()),
        ("privates.ts", "console.log(\"#addRoute|", public()),
    ];
    for (file, code, want) in cases {
        let mut a = fixture_app("typescript");
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{file}: {code}");
    }
    // A JSX tag's `>` closes no header: the parameter of an attribute's callback is not the
    // children's `repo`.
    let (dir, mut a) = project_app(
        "jsx",
        &[
            (
                "repos.ts",
                "export class UserRepository {\n  deleteUser(id: number): void {}\n}\nexport class AuditLog {\n  deleteUser(id: number): void {}\n}\n",
            ),
            (
                "page.tsx",
                "import { AuditLog, UserRepository } from \"./repos\";\n\nexport function Page(repo: UserRepository, id: number) {\n  return (\n    <List\n      title=\"every user of the long named list\"\n      render={(repo: AuditLog) => repo.deleteUser(id)}\n    >\n      {repo.deleteUser(id + 1)}\n    </List>\n  );\n}\n",
            ),
        ],
    );
    a.external
        .insert(Kind::TsJs, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "page.tsx", "{repo.deleteUser|(id + 1)");
    assert_eq!(
        shown(&mut a),
        jump(
            "deleteUser \u{2192} UserRepository.deleteUser (via repo: UserRepository)",
            "repos.ts:2"
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
