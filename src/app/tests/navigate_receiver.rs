//! `d` on a member: found by name, or in the type of the receiver.

use super::*;

/// Steps 7 and 1 of #68 over the same project in three languages: on `x.word` with `x` a
/// value whose type is not known, one member declaration of the name jumps and says it was
/// found by name; several open a picker whose rows say what each is declared in and why it
/// is there.
#[test]
fn a_member_of_a_value_is_found_by_name_and_says_so() {
    let two =
        |status: &str, first: (&str, &str), second: (&str, &str)| picker(status, &[first, second]);
    let cases: [(&str, &str, &str, Shown); 6] = [
        // A parameter with no annotation.
        (
            "python",
            "factories.py",
            "repo.find_user",
            jump(
                "find_user \u{2192} UserRepository.find_user (by name, 1 match)",
                "repos.py:5",
            ),
        ),
        // Two classes declare the method: a picker, never a guess.
        (
            "python",
            "factories.py",
            "return repo.delete_user",
            two(
                "delete_user: by name, 2 declarations",
                ("UserRepository.delete_user", "repos.py:8"),
                ("AuditLog.delete_user", "repos.py:13"),
            ),
        ),
        // `any` is no type of the project.
        (
            "typescript",
            "factories.ts",
            "repo.findUser",
            jump(
                "findUser \u{2192} UserRepository.findUser (by name, 1 match)",
                "repos.ts:6",
            ),
        ),
        (
            "typescript",
            "factories.ts",
            "void repo.deleteUser",
            two(
                "deleteUser: by name, 2 declarations",
                ("UserRepository.deleteUser", "repos.ts:10"),
                ("AuditLog.deleteUser", "repos.ts:16"),
            ),
        ),
        // The variable of a `range` over a channel, which the rules do not read.
        (
            "go",
            "factories.go",
            "repo.FindUser",
            jump(
                "FindUser \u{2192} UserRepository.FindUser (by name, 1 match)",
                "repos.go:11",
            ),
        ),
        (
            "go",
            "factories.go",
            "^\t\trepo.DeleteUser",
            two(
                "DeleteUser: by name, 2 declarations",
                ("UserRepository.DeleteUser", "repos.go:15"),
                ("AuditLog.DeleteUser", "repos.go:21"),
            ),
        ),
    ];
    for (fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{fixture}: {code}");
    }
    // The picker is titled with what the status line says.
    let mut a = fixture_app("go");
    d_on(&mut a, "factories.go", "^\t\trepo.DeleteUser");
    assert_eq!(
        a.picker.as_ref().unwrap().title,
        "DeleteUser: by name, 2 declarations"
    );
}

/// Found by the acceptance pass of #68. On a declaration, the other declarations of the name
/// are namesakes nothing ties to it: a second `d` after a proven jump used to leave
/// `UserRepository.delete_user` for `AuditLog.delete_user` on its own, with `1 match`. They
/// are offered in a picker that says where the cursor stands, even when there is one.
#[test]
fn a_declaration_does_not_jump_to_its_namesake() {
    let cases = [
        (
            "python",
            "repos.py",
            "async def delete_user",
            "delete_user",
            "AuditLog.delete_user",
            "repos.py:13",
        ),
        (
            "typescript",
            "repos.ts",
            "async deleteUser",
            "deleteUser",
            "AuditLog.deleteUser",
            "",
        ),
        (
            "go",
            "repos.go",
            ") DeleteUser",
            "DeleteUser",
            "AuditLog.DeleteUser",
            "repos.go:21",
        ),
    ];
    for (fixture, file, code, word, other, place) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        let Shown::Picker(status, rows) = shown(&mut a) else {
            panic!("{fixture}: a jump to {}", a.message);
        };
        assert_eq!(
            status,
            format!("{word}: at a declaration, 1 other by name"),
            "{fixture}"
        );
        assert_eq!(rows.len(), 1, "{fixture}");
        assert_eq!(rows[0].0, other, "{fixture}");
        assert!(
            place.is_empty() || rows[0].2 == place,
            "{fixture}: {}",
            rows[0].2
        );
    }
}

/// Steps 2 and 3 of #68 over the same project in three languages. A receiver whose every
/// declaration in scope reads one type, directly or through the return type of one call, has
/// its member looked up in that type: one jump, which says the link it followed. Two
/// declarations that disagree, or a call whose return type is not written, leave the member
/// to the search by name: a picker of two.
#[test]
fn a_member_of_a_typed_receiver_is_looked_up_in_its_type() {
    let cases: Vec<(&str, &str, &str, Shown)> = vec![
        // Two same-named methods: each field lands on its own class's.
        (
            "python",
            "service.py",
            "self.repo.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via self.repo: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "service.py",
            "self.audit.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via self.audit: AuditLog)",
                "repos.py:13",
            ),
        ),
        // The declared type's method, not its implementations (step 6).
        (
            "python",
            "service.py",
            "self.notifier.send",
            jump(
                "send \u{2192} Notifier.send (via self.notifier: Notifier)",
                "repos.py:18",
            ),
        ),
        // Shadowing: below the nested function only the outer `repo` is in scope; inside it
        // the inner one hides it (#100).
        (
            "python",
            "service.py",
            "^    repo.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via repo: AuditLog)",
                "repos.py:13",
            ),
        ),
        (
            "python",
            "service.py",
            "await repo.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via repo: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "factories.py",
            "repo.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via make_repo() -> UserRepository)",
                "repos.py:8",
            ),
        ),
        // No return type, but every `return` constructs one (#100).
        (
            "python",
            "factories.py",
            "audit.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via make_audit() returns AuditLog())",
                "repos.py:13",
            ),
        ),
        // The return type is resolved where the function is declared.
        (
            "python",
            "factories.py",
            "session.close",
            jump(
                "close \u{2192} Session.close (via connect() -> Session)",
                "store/sessions.py:6",
            ),
        ),
        (
            "typescript",
            "service.ts",
            "this.repo.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via this.repo: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "service.ts",
            "this.audit.deleteUser",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via this.audit: AuditLog)",
                "repos.ts:16",
            ),
        ),
        (
            "typescript",
            "service.ts",
            "this.notifier.send",
            jump(
                "send \u{2192} Notifier.send (via this.notifier: Notifier)",
                "repos.ts:22",
            ),
        ),
        (
            "typescript",
            "service.ts",
            "^  repo.deleteUser",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via repo: AuditLog)",
                "repos.ts:16",
            ),
        ),
        (
            "typescript",
            "service.ts",
            "await repo.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via repo: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "factories.ts",
            "await repo.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via makeRepo(): UserRepository)",
                "repos.ts:10",
            ),
        ),
        // No return type, but the body constructs one.
        (
            "typescript",
            "factories.ts",
            "audit.deleteUser",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via makeAudit() returns new AuditLog())",
                "repos.ts:16",
            ),
        ),
        (
            "typescript",
            "factories.ts",
            "session.close",
            jump(
                "close \u{2192} Session.close (via connect(): Session)",
                "store/sessions.ts:6",
            ),
        ),
        (
            "go",
            "service.go",
            "s.repo.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via s.repo: UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "service.go",
            "s.audit.DeleteUser",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via s.audit: AuditLog)",
                "repos.go:21",
            ),
        ),
        // An interface's method line.
        (
            "go",
            "service.go",
            "s.notifier.Send",
            jump(
                "Send \u{2192} Notifier.Send (via s.notifier: Notifier)",
                "repos.go:26",
            ),
        ),
        (
            "go",
            "service.go",
            "^\trepo.DeleteUser",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via repo: AuditLog)",
                "repos.go:21",
            ),
        ),
        (
            "go",
            "service.go",
            "^\t\trepo.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via repo: UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "factories.go",
            "repo.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via NewRepo() *UserRepository)",
                "repos.go:15",
            ),
        ),
        // The first result of two.
        (
            "go",
            "factories.go",
            "audit.DeleteUser",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via NewAudit() AuditLog)",
                "repos.go:21",
            ),
        ),
        // A function of an imported package, whose result is a type of that package.
        (
            "go",
            "factories.go",
            "session.Close",
            jump(
                "Close \u{2192} Session.Close (via store.Open() *Session)",
                "store/store.go:15",
            ),
        ),
    ];
    for (fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
    }
}

/// Step 4 of #68 over the same project in three languages. A chain is followed one field at a
/// time, each through the type before it (a Go field may be promoted from an embedded struct),
/// and a jump lists the links. A link that cannot be proven, or a seventh name, falls back to
/// the search by name and says where the chain broke. A chain that hangs off a call has no
/// names to follow.
#[test]
fn a_chain_is_followed_link_by_link() {
    let by_name = |status: &str, rows: &[(&str, &str)]| picker(status, rows);
    let folders = |root: &str, field: &str, ty: &str| {
        let tail = format!(" \u{2192} {field}: {ty}").repeat(4);
        format!("{root} \u{2192} {ty}.{root} (via folder.{field}: {ty}{tail})")
    };
    let py_both = [
        ("UserRepository.delete_user", "repos.py:8"),
        ("AuditLog.delete_user", "repos.py:13"),
    ];
    let ts_both = [
        ("UserRepository.deleteUser", "repos.ts:10"),
        ("AuditLog.deleteUser", "repos.ts:16"),
    ];
    let go_both = [
        ("UserRepository.DeleteUser", "repos.go:15"),
        ("AuditLog.DeleteUser", "repos.go:21"),
    ];
    let cases: Vec<(&str, &str, &str, Shown)> = vec![
        // A parameter, then a field handed on from a constructor parameter, twice.
        (
            "python",
            "service.py",
            "app.services.users.remove",
            jump(
                "remove \u{2192} UserService.remove (via app.services: Services \u{2192} users: UserService)",
                "service.py:10",
            ),
        ),
        // Two same-named methods: each field of the unit of work lands on its own.
        (
            "python",
            "chains.py",
            "self.uow.users.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via self.uow: UnitOfWork \u{2192} users: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "chains.py",
            "self.uow.audit.delete_user",
            jump(
                "delete_user \u{2192} AuditLog.delete_user (via self.uow: UnitOfWork \u{2192} audit: AuditLog)",
                "repos.py:13",
            ),
        ),
        // A type parameter is not the type argument.
        (
            "python",
            "chains.py",
            "self.box.item.delete_user",
            by_name(
                "delete_user: by name, 2 declarations (chain broke at item)",
                &py_both,
            ),
        ),
        // Not the local `users`: the chain hangs off a call, whose declared return type
        // starts it (#100).
        (
            "python",
            "chains.py",
            "make_uow().users.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via make_uow() -> UnitOfWork \u{2192} users: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "chains.py",
            "folder.parent.parent.parent.parent.parent.root",
            jump(&folders("root", "parent", "Folder"), "chains.py:23"),
        ),
        (
            "python",
            "chains.py",
            "folder.parent.parent.parent.parent.parent.parent.root",
            jump(
                "root \u{2192} Folder.root (by name, 1 match, chain broke at parent)",
                "chains.py:23",
            ),
        ),
        // A `@cached_property` with a return type is a field of that type; a `@property`
        // without one is not read, nor the typed one of the base class it overrides.
        (
            "python",
            "chains.py",
            "self.registry.users.delete_user",
            jump(
                "delete_user \u{2192} UserRepository.delete_user (via self.registry: Registry \u{2192} users: UserRepository)",
                "repos.py:8",
            ),
        ),
        (
            "python",
            "chains.py",
            "self.registry.audit.delete_user",
            by_name(
                "delete_user: by name, 2 declarations (chain broke at audit)",
                &py_both,
            ),
        ),
        (
            "typescript",
            "service.ts",
            "app.services.users.remove",
            jump(
                "remove \u{2192} UserService.remove (via app.services: Services \u{2192} users: UserService)",
                "service.ts:11",
            ),
        ),
        (
            "typescript",
            "chains.ts",
            "this.uow.users.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via this.uow: UnitOfWork \u{2192} users: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "chains.ts",
            "this.uow.audit.deleteUser",
            jump(
                "deleteUser \u{2192} AuditLog.deleteUser (via this.uow: UnitOfWork \u{2192} audit: AuditLog)",
                "repos.ts:16",
            ),
        ),
        (
            "typescript",
            "chains.ts",
            "this.box.item.deleteUser",
            by_name(
                "deleteUser: by name, 2 declarations (chain broke at item)",
                &ts_both,
            ),
        ),
        (
            "typescript",
            "chains.ts",
            "makeUow().users.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via makeUow(): UnitOfWork \u{2192} users: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "chains.ts",
            "folder.parent.parent.parent.parent.parent.root",
            jump(&folders("root", "parent", "Folder"), "chains.ts:15"),
        ),
        (
            "typescript",
            "chains.ts",
            "folder.parent.parent.parent.parent.parent.parent.root",
            jump(
                "root \u{2192} Folder.root (by name, 1 match, chain broke at parent)",
                "chains.ts:15",
            ),
        ),
        (
            "typescript",
            "chains.ts",
            "this.registry.users.deleteUser",
            jump(
                "deleteUser \u{2192} UserRepository.deleteUser (via this.registry: Registry \u{2192} users: UserRepository)",
                "repos.ts:10",
            ),
        ),
        (
            "typescript",
            "chains.ts",
            "this.registry.audit.deleteUser",
            by_name(
                "deleteUser: by name, 2 declarations (chain broke at audit)",
                &ts_both,
            ),
        ),
        // Pointer fields.
        (
            "go",
            "service.go",
            "app.Services.Users.Remove",
            jump(
                "Remove \u{2192} UserService.Remove (via app.Services: Services \u{2192} Users: UserService)",
                "service.go:11",
            ),
        ),
        // A field promoted from the embedded `*Deps`.
        (
            "go",
            "chains.go",
            "h.uow.Users.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via h.uow: UnitOfWork \u{2192} Users: UserRepository)",
                "repos.go:15",
            ),
        ),
        // The embedded struct named as a link.
        (
            "go",
            "chains.go",
            "h.Deps.uow.Audit.DeleteUser",
            jump(
                "DeleteUser \u{2192} AuditLog.DeleteUser (via h.Deps: Deps \u{2192} uow: UnitOfWork \u{2192} Audit: AuditLog)",
                "repos.go:21",
            ),
        ),
        (
            "go",
            "chains.go",
            "h.box.Item.DeleteUser",
            by_name(
                "DeleteUser: by name, 2 declarations (chain broke at Item)",
                &go_both,
            ),
        ),
        (
            "go",
            "chains.go",
            "NewUnitOfWork().Users.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via NewUnitOfWork() *UnitOfWork \u{2192} Users: UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "go",
            "chains.go",
            "folder.Parent.Parent.Parent.Parent.Parent.Root",
            jump(&folders("Root", "Parent", "Folder"), "chains.go:16"),
        ),
        (
            "go",
            "chains.go",
            "folder.Parent.Parent.Parent.Parent.Parent.Parent.Root",
            jump(
                "Root \u{2192} Folder.Root (by name, 1 match, chain broke at Parent)",
                "chains.go:16",
            ),
        ),
    ];
    for (fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
    }
}
