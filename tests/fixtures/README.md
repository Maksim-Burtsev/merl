# Go-to-definition fixtures

The same small project in Python, TypeScript and Go, for the tests of `d` (#68). Each has:

- two methods with the same name in different classes: `UserRepository.delete_user` and
  `AuditLog.delete_user`;
- a shadowed variable: `repo` in `cleanup` is an `AuditLog`, and inside `purge` a
  `UserRepository`;
- a chain: `app.services.users.remove`;
- an interface with two implementations: `Notifier.send`, implemented by `EmailNotifier` and
  `SmsNotifier`;
- imports inside the project (step 5): `jobs` imports `repos` and the `store` package, which
  imports its `sessions` module relatively (and, in TypeScript, `jobs` reaches it through the
  `@/` alias of `tsconfig.json`). `fakes` declares a second `UserRepository` and a second
  `open_session` that the imports must not pick, `store` only re-exports `open_session`, and
  `sessions` has two classes with a `start` (Go: a function `Open` and a method `Session.Open`).
- receiver types (steps 2 and 3): `service` declares its fields by annotation, construction and
  constructor parameter, so `repo` and `audit` each lead to their own `delete_user`, and the
  `repo` of `cleanup` is shadowed inside `purge`. `factories` assigns receivers from calls: with a
  declared return type (`make_repo`, `NewRepo`, Go's first result of `NewAudit`), without one
  (Python's `make_audit`; TypeScript's `makeAudit` returns `new AuditLog()`), and through
  `connect()` / `store.Open()`, whose `Session` is declared in the `store` package. The receiver of
  `forget` has a type the rules cannot read.
- chains (step 4): `chains` has a `UnitOfWork` whose `users` and `audit` lead to their own
  `delete_user`, reached through a field of `Handler` (Go: through the embedded `*Deps`, promoted
  and by name). `box.item` is typed by a generic parameter, so that chain breaks at `item`.
  `folder.parent…` is followed for six names and breaks at the seventh. `make_uow().users` hangs
  off a call next to a local `users` of another type, which must not be read as its receiver.

Each step of #68 adds the cases it resolves to the tests over these files. A later step can
change what `d` shows on a line here, but the files stay the same shape in all three languages.
