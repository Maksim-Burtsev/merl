//! `d` through the return type of a call, and through a cast.

use super::*;

/// #100: one more hop on a call. A method called on a receiver whose type is proven gives
/// what it declares to return, a Python function or method with no annotation what every
/// `return` of it constructs, and a chain may hang off a call that starts the expression.
/// A call of a call, returns that differ and an unproven receiver stay by name.
#[test]
fn a_call_is_one_more_hop() {
    let cases: Vec<(&str, &str, &str, Shown)> = vec![
        // A method of a parameter whose type is written, with the return type it declares.
        (
            "python",
            "calls.py",
            "repo.delete_user|(user_id)",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via depot.people_repo() -> UserRepository)",
                "repos.py:8",
            ),
        ),
        // No annotation, and every `return` constructs an `AuditLog`.
        (
            "python",
            "calls.py",
            "trail.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via depot.trail() returns AuditLog())",
                "repos.py:13",
            ),
        ),
        // A chain off a call that starts the expression: a function, a class, and the field itself.
        (
            "python",
            "calls.py",
            "open_depot().people.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via open_depot() -> Depot \u{2192} people: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "calls.py",
            "Depot().people.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via Depot(): Depot \u{2192} people: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "calls.py",
            "open_depot().people|.delete_user",
            jump(
                "people \u{2192} Depot.people (via open_depot() -> Depot)",
                "calls.py:8",
            ),
        ),
        // The returns differ; one of them is `None`; a decorator may return anything.
        (
            "python",
            "calls.py",
            "either.delete_user",
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
            "calls.py",
            "maybe.delete_user",
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
            "calls.py",
            "shared.delete_user",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // A call of a call hangs off a value nobody typed.
        (
            "python",
            "calls.py",
            "open_depot().people_repo().delete_user",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // The receiver of the method is a parameter with no annotation.
        (
            "python",
            "calls.py",
            "repo.delete_user|(user_id + 8)",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // TypeScript: a method's `): T`, a method whose every `return` is `new T()`, a chain off a
        // function and off `new`, and the method line of an interface.
        (
            "typescript",
            "calls.ts",
            "repo.deleteUser|(id);",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via depot.peopleRepo(): UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "calls.ts",
            "trail.deleteUser",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via depot.trail() returns new AuditLog())",
                "repos.ts:16",
            ),
        ),
        (
            "typescript",
            "calls.ts",
            "openDepot().people.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via openDepot(): Depot \u{2192} people: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "calls.ts",
            "new Depot().people.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via new Depot(): Depot \u{2192} people: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "calls.ts",
            "sourced.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via src.source(): UserRepository)",
                "repos.ts:10",
            ),
        ),
        // Overloads declare the method three times: which one is called is not read.
        (
            "typescript",
            "calls.ts",
            "picked.deleteUser",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        // A call of a call, and a receiver typed `any`.
        (
            "typescript",
            "calls.ts",
            "openDepot().peopleRepo().deleteUser",
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
            "calls.ts",
            "repo.deleteUser|(id + 6)",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        // Go: a method behind its receiver, the first of two results, a chain off a function and
        // off a package's, and the method line of an interface.
        (
            "go",
            "calls.go",
            "repo.DeleteUser|(id)",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via depot.PeopleRepo() *UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "calls.go",
            "trail.DeleteUser",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via depot.Trail() AuditLog)",
                "repos.go:21",
            ),
        ),
        (
            "go",
            "calls.go",
            "OpenDepot().People.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via OpenDepot() *Depot \u{2192} People: UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "calls.go",
            "store.Open().Close",
            jump(
                "Close \u{2192} Session.Close (via store.Open() *Session)",
                "store/store.go:15",
            ),
        ),
        (
            "go",
            "calls.go",
            "sourced.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via src.Source() *UserRepository)",
                "repos.go:15",
            ),
        ),
        // A call of a call, and a receiver that is the second name of a `:=`.
        (
            "go",
            "calls.go",
            "OpenDepot().PeopleRepo().DeleteUser",
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
            "calls.go",
            "repo.DeleteUser|(id + 5)",
            picker(
                "DeleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.DeleteUser", "repos.go:15"),
                    ("AuditLog.DeleteUser", "repos.go:21"),
                ],
            ),
        ),
        // A decorator written over several lines may return anything too.
        (
            "python",
            "calls.py",
            "wrapped.delete_user",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // A parameter named like a function of the module is a value nobody typed.
        (
            "python",
            "calls.py",
            "open_depot().people.delete_user|(user_id + 10)",
            picker(
                "delete_user: by name, 2 declarations (chain broke at open_depot())",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        (
            "python",
            "calls.py",
            "made.people.delete_user",
            picker(
                "delete_user: by name, 2 declarations (chain broke at made)",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // The method is inherited; the receiver is a field; the chain hangs off a method
        // of a proven receiver.
        (
            "python",
            "calls.py",
            "inherited.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via sub.people_repo() -> UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "calls.py",
            "through.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via self.depot.people_repo() -> UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "calls.py",
            "self.depot.people_repo().delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via self.depot.people_repo() -> UserRepository)",
                "repos.py:8",
            ),
        ),
        // Two locals assigned from each other's methods end in a picker, and end.
        (
            "python",
            "calls.py",
            "ahead.delete_user",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // A `return` behind an `if` on its own line is a return the rules do not read.
        (
            "python",
            "calls.py",
            "picked.delete_user",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        (
            "typescript",
            "calls.ts",
            "inline.deleteUser",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
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

/// #100: a cast writes the type. `typing.cast(T, x)`, `x as T`, `v, ok := i.(T)`, and the
/// variable of `switch v := x.(type)` inside a `case T:`, assigned to a name or with the chain
/// hanging off the cast itself. A cast to a type the project does not declare, a `case` of
/// several types and `default` prove nothing.
#[test]
fn a_cast_writes_the_type() {
    let cases: Vec<(&str, &str, &str, Shown)> = vec![
        // `cast(T, x)` and `typing.cast("T", x)` assigned to a name, and with the member hanging
        // off the cast itself.
        (
            "python",
            "casts.py",
            "repo.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "casts.py",
            "audit.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via audit: AuditLog)",
                "repos.py:13",
            ),
        ),
        (
            "python",
            "casts.py",
            "cast(UserRepository, found).delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via cast(UserRepository, found))",
                "repos.py:8",
            ),
        ),
        // A cast to a type the project does not declare proves nothing.
        (
            "python",
            "casts.py",
            "missing.delete_user",
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
            "casts.py",
            "loose.delete_user",
            picker(
                "delete_user: by name, 2 declarations",
                &[
                    ("UserRepository.delete_user", "repos.py:8"),
                    ("AuditLog.delete_user", "repos.py:13"),
                ],
            ),
        ),
        // TypeScript: `x as T`, the last type of `x as unknown as T`, and `(x as T).member`.
        (
            "typescript",
            "casts.ts",
            "repo.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via repo: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "casts.ts",
            "audit.deleteUser",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via audit: AuditLog)",
                "repos.ts:16",
            ),
        ),
        (
            "typescript",
            "casts.ts",
            "(found as UserRepository).deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via found as UserRepository)",
                "repos.ts:10",
            ),
        ),
        // `any`, and a generic wrapper that is no type of the project.
        (
            "typescript",
            "casts.ts",
            "loose.deleteUser",
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
            "casts.ts",
            "partial.deleteUser",
            picker(
                "deleteUser: by name, 2 declarations",
                &[
                    ("UserRepository.deleteUser", "repos.ts:10"),
                    ("AuditLog.deleteUser", "repos.ts:16"),
                ],
            ),
        ),
        // Go: `v, ok := i.(T)`, `v := i.(T)` and `i.(T).Member`.
        (
            "go",
            "casts.go",
            "repo.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via repo: UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "casts.go",
            "audit.DeleteUser",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via audit: AuditLog)",
                "repos.go:21",
            ),
        ),
        (
            "go",
            "casts.go",
            "found.(*UserRepository).DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via found.(*UserRepository))",
                "repos.go:15",
            ),
        ),
        // `fmt.Stringer` is declared outside the project.
        (
            "go",
            "casts.go",
            "found.(fmt.Stringer).String",
            jump("no definition for String", "casts.go:25"),
        ),
        // The variable of a type switch has the type of the `case` the cursor is in, also from a
        // block inside it.
        (
            "go",
            "casts.go",
            "v.DeleteUser|(id + 3)",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via v: UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "casts.go",
            "v.DeleteUser|(id + 4)",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via v: AuditLog)",
                "repos.go:21",
            ),
        ),
        // A second type switch below the first: the first one's `v` is out of scope.
        (
            "go",
            "casts.go",
            "return v.Area|()",
            jump("Area \u{2192} Square.Area (via v: Square)", "casts.go:11"),
        ),
        // A `case` of two types and `default` leave `v` what it was.
        (
            "go",
            "casts.go",
            "v.Area|() + 1",
            picker(
                "Area: by name, 2 declarations",
                &[
                    ("Square.Area", "casts.go:11"),
                    ("Circle.Area", "casts.go:15"),
                ],
            ),
        ),
        (
            "go",
            "casts.go",
            "v.Area|() + 2",
            picker(
                "Area: by name, 2 declarations",
                &[
                    ("Square.Area", "casts.go:11"),
                    ("Circle.Area", "casts.go:15"),
                ],
            ),
        ),
        // A `cast` the project declares itself is a function with a return type.
        (
            "python",
            "casts_own.py",
            "repo.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via cast() -> AuditLog)",
                "repos.py:13",
            ),
        ),
    ];
    for (fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
    }
}
