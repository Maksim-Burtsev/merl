//! `d` on a value: which binding wins, and what it was assigned.

use super::*;

/// #100: the innermost scope that binds a name decides what it is. A module-level or
/// package-level name, a variable of the function around a closure and one of the block
/// around a block are hidden, where they used to disagree with the inner one. What hides may
/// be unknown, and then nothing behind it answers; two bindings of one scope still disagree.
#[test]
fn the_innermost_binding_wins() {
    let cases: Vec<(&str, &str, &str, Shown)> = vec![
        // A parameter named like a module-level `def`.
        (
            "python",
            "scopes.py",
            "save.find_user",
            jump(
                "find_user \u{2192} UserRepository.find_user (via save: UserRepository)",
                "repos.py:5",
            ),
        ),
        // A local hides the module's variable, in the function and in a closure inside it.
        (
            "python",
            "scopes.py",
            "ledger.find_user|(user_id)",
            jump(
                "find_user \u{2192} UserRepository.find_user (via ledger: UserRepository)",
                "repos.py:5",
            ),
        ),
        (
            "python",
            "scopes.py",
            "ledger.find_user|(user_id + 1)",
            jump(
                "find_user \u{2192} UserRepository.find_user (via ledger: UserRepository)",
                "repos.py:5",
            ),
        ),
        // A function that binds no `ledger` reads the module's.
        (
            "python",
            "scopes.py",
            "ledger.delete_user|(user_id + 2)",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via ledger: AuditLog)",
                "repos.py:13",
            ),
        ),
        // Two bindings in one function disagree: which one reaches the line is control flow.
        (
            "python",
            "scopes.py",
            "ledger.delete_user|(3)",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // A parameter with no annotation hides the module's `ledger`, which must not answer.
        (
            "python",
            "scopes.py",
            "ledger.delete_user|(user_id + 4)",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // On the name itself, the declaration in its scope.
        (
            "python",
            "scopes.py",
            "    ledger|.find_user(user_id)",
            jump("ledger \u{2192} rotate.ledger (local)", "scopes.py:15"),
        ),
        // TypeScript: a `const` of the function, an arrow function's parameter, a block's `const`
        // over the function's, and the function's own below that block.
        (
            "typescript",
            "scopes.ts",
            "ledger.findUser",
            jump(
                "findUser \u{2192} UserRepository.findUser (via ledger: UserRepository)",
                "repos.ts:6",
            ),
        ),
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id);",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 1)",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 2)",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via ledger: AuditLog)",
                "repos.ts:16",
            ),
        ),
        // `any` hides the module's `ledger`.
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 3)",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        // The cursor is not inside an arrow function on its own line, or on the line the
        // statement started on: its parameter counts and hides nothing.
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(repos.map",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 4)",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        (
            "typescript",
            "scopes.ts",
            "  ledger|.findUser(id)",
            jump("ledger \u{2192} rotate.ledger (local)", "scopes.ts:6"),
        ),
        // A literal under a line that also holds an arrow function: the cursor is in the
        // literal, and the arrow's parameter hides nothing.
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 5)",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        // Go: a `:=` over the package's `var`, a block's over the function's, and the package's
        // where nothing hides it.
        (
            "go",
            "scopes.go",
            "ledger.FindUser",
            jump(
                "FindUser \u{2192} UserRepository.FindUser (via NewRepo() *UserRepository)",
                "repos.go:11",
            ),
        ),
        (
            "go",
            "scopes.go",
            "ledger.DeleteUser|(id + 1)",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via NewRepo() *UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "scopes.go",
            "ledger.DeleteUser|(id + 2)",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via ledger: AuditLog)",
                "repos.go:21",
            ),
        ),
        (
            "go",
            "scopes.go",
            "ledger.DeleteUser|(id + 3)",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via ledger: AuditLog)",
                "repos.go:21",
            ),
        ),
        // The second name of a `:=` is unknown and hides the package's.
        (
            "go",
            "scopes.go",
            "ledger.DeleteUser|(id + 4)",
            picker(
                "DeleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.DeleteUser", "repos.go:15"),
                    ("AuditLog.DeleteUser", "repos.go:21"),
                ],
            ),
        ),
        // An `if` header and its body are read as one scope, so their two `ledger` disagree.
        (
            "go",
            "scopes.go",
            "ledger.DeleteUser|(id + 5)",
            picker(
                "DeleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.DeleteUser", "repos.go:15"),
                    ("AuditLog.DeleteUser", "repos.go:21"),
                ],
            ),
        ),
        (
            "go",
            "scopes.go",
            "	ledger|.FindUser(id)",
            jump(
                "ledger \u{2192} ScopedRotate.ledger (local)",
                "scopes.go:10",
            ),
        ),
        // A composite literal under a line that also holds a `func`.
        (
            "go",
            "scopes.go",
            "ledger.DeleteUser|(id + 6)",
            picker(
                "DeleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.DeleteUser", "repos.go:15"),
                    ("AuditLog.DeleteUser", "repos.go:21"),
                ],
            ),
        ),
        // A sibling block's callback or loop is not around the cursor of the `else` or the
        // `catch`, which reads the parameter of its own function. A callback on the lines
        // of the header itself counts, as one on the cursor's line does, and hides nothing.
        (
            "go",
            "scopes.go",
            "ledger.DeleteUser|(7)",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via ledger: AuditLog)",
                "repos.go:21",
            ),
        ),
        (
            "go",
            "scopes.go",
            "ledger.DeleteUser|(8)",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via ledger: AuditLog)",
                "repos.go:21",
            ),
        ),
        (
            "go",
            "scopes.go",
            "ledger.DeleteUser|(9)",
            picker(
                "DeleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.DeleteUser", "repos.go:15"),
                    ("AuditLog.DeleteUser", "repos.go:21"),
                ],
            ),
        ),
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 6)",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via ledger: AuditLog)",
                "repos.ts:16",
            ),
        ),
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 7)",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via ledger: AuditLog)",
                "repos.ts:16",
            ),
        ),
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 8)",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 9",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        // A docstring's example binds nothing: the module's `ledger` is read.
        (
            "python",
            "scopes.py",
            "ledger.delete_user|(user_id + 5)",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via ledger: AuditLog)",
                "repos.py:13",
            ),
        ),
        // The arrow function's line ends in `=>`: its body is the lines below.
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 10)",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                "repos.ts:10",
            ),
        ),
        // A destructured parameter under a header closed by `}: Deps): void {` hides the
        // module's `ledger` and has no type of its own.
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 11)",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        // A backtick inside a regex opens no template: the `const` under it is read.
        (
            "typescript",
            "scopes.ts",
            "ledger.deleteUser|(id + 12)",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via ledger: UserRepository)",
                "repos.ts:10",
            ),
        ),
    ];
    for (fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
    }
}

/// #100: a loop variable is an element of what it loops over, where that collection's type is
/// written: `list[T]`, `T[]`, `[]T`, `map[K]T`, as an annotation or as the return type of the
/// function it came from. The collection itself is no `T`, and keys, pairs, a tuple target
/// and a collection declared twice stay unknown.
#[test]
fn a_loop_variable_is_an_element_of_a_written_collection() {
    let cases: Vec<(&str, &str, &str, Shown)> = vec![
        // An annotated parameter: `list[T]`, and `tuple[T, ...]` in quotes.
        (
            "python",
            "elements.py",
            "repo.delete_user|(user_id)",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via repos: list[UserRepository])",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "elements.py",
            "log.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via logs: \"tuple[AuditLog, ...]\")",
                "repos.py:13",
            ),
        ),
        // The collection came from a function that declares what it returns.
        (
            "python",
            "elements.py",
            "repo.delete_user|(user_id + 3)",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via load_repos() -> list[UserRepository])",
                "repos.py:8",
            ),
        ),
        // The list itself is no `UserRepository`.
        (
            "python",
            "elements.py",
            "repos.delete_user",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // A `dict` hands out its keys; a tuple target over a call is not read.
        (
            "python",
            "elements.py",
            "key.delete_user",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        (
            "python",
            "elements.py",
            "repo.delete_user|(user_id + 5)",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // `repos` is assigned again with no type written: not every declaration says what it holds.
        (
            "python",
            "elements.py",
            "repo.delete_user|(user_id + 6)",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // Two annotations that disagree.
        (
            "python",
            "elements.py",
            "repo.delete_user|(user_id + 7)",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // TypeScript: `T[]`, `ReadonlyArray<T>`, a declared return type.
        (
            "typescript",
            "elements.ts",
            "repo.deleteUser|(id);",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via repos: UserRepository[])",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "elements.ts",
            "log.deleteUser",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via logs: ReadonlyArray<AuditLog>)",
                "repos.ts:16",
            ),
        ),
        (
            "typescript",
            "elements.ts",
            "repo.deleteUser|(id + 2)",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via loadRepos(): UserRepository[])",
                "repos.ts:10",
            ),
        ),
        // The array itself, a `Map`'s pairs and the keys of `for … in`.
        (
            "typescript",
            "elements.ts",
            "repos.deleteUser",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        (
            "typescript",
            "elements.ts",
            "entry.deleteUser",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        (
            "typescript",
            "elements.ts",
            "index.deleteUser",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        // Go: the second variable of a `range` over `[]*T`, over `map[K]T`, over a declared result.
        (
            "go",
            "elements.go",
            "repo.DeleteUser|(id)",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via repos: []*UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "elements.go",
            "entry.DeleteUser",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via logs: map[string]AuditLog)",
                "repos.go:21",
            ),
        ),
        (
            "go",
            "elements.go",
            "repo.DeleteUser|(id + 2)",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via LoadRepos() []*UserRepository)",
                "repos.go:15",
            ),
        ),
        // A single variable is an index or a key, or the element of a channel the rules do not read.
        (
            "go",
            "elements.go",
            "repo.DeleteUser|(id + 3)",
            picker(
                "DeleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.DeleteUser", "repos.go:15"),
                    ("AuditLog.DeleteUser", "repos.go:21"),
                ],
            ),
        ),
        // A slice made or written out in place says what it holds.
        (
            "go",
            "elements.go",
            "repo.DeleteUser|(id + 4)",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via made: []*UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "elements.go",
            "entry.DeleteUser|(id + 5)",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via listed: []AuditLog)",
                "repos.go:21",
            ),
        ),
        // A named slice type, read where it is declared.
        (
            "go",
            "elements.go",
            "repo.DeleteUser|(id + 6)",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via repos: RepoList)",
                "repos.go:15",
            ),
        ),
        // An element handed on to another name is one hop too many.
        (
            "python",
            "elements.py",
            "current.delete_user",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // A named map, a named slice declared in another package, and a `type X = []T`
        // alias, which is not read.
        (
            "go",
            "elements.go",
            "repo.DeleteUser|(id + 7)",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via index: RepoIndex)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "elements.go",
            "session.Close",
            jump(
                "Close \u{2192} Session.Close (via sessions: store.SessionList)",
                "store/store.go:15",
            ),
        ),
        (
            "go",
            "elements.go",
            "repo.DeleteUser|(id + 8)",
            picker(
                "DeleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.DeleteUser", "repos.go:15"),
                    ("AuditLog.DeleteUser", "repos.go:21"),
                ],
            ),
        ),
    ];
    for (fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
    }
}
