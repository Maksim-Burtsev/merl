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
  `Admin` reaches a `Registry` whose `users` is a `@cached_property` / a getter with a return type,
  a link, and whose `audit` overrides a typed one of `BaseRegistry` with none, so the chain breaks
  there rather than reading the base's type. Go has no properties, so `chains.go` has no such link.

- implementations (step 6): `impls` carries a `BaseJob` whose `run` two subclasses override and a
  third inherits, a `NightlyJob` a level below one of them, a `Sweeper` with a single
  implementation, and a `LoudNotifier` that implements the `Notifier` of `repos` from another file.
  Python's `Notifier` is a `Protocol` and Go's is an interface, so both are answered structurally:
  `WebhookNotifier` implements the protocol without naming it, and `Batch.send` / `Batch.Run` take
  one parameter too many to be an implementation of anything.

- fields (#104): `fields` has an `Issue` that extends a `Base` (Go: embeds it) and declares its
  fields in every form the rules read — a class-body annotation with a value and without, a
  constructor parameter handed on (TypeScript: of a constructor wrapped over several lines), a
  field assigned again in another method, `Title, Body string` — and a `Comment` sharing
  `poster_id` with it and alone declaring `body` (Python: under a docstring whose `Attributes`
  section spells `body : str`), for the search by name. `Issue` assigns the `audit` of `Base`
  again, which must not make it `Issue`'s; Python's `Issue.summary` is a field over a method of
  `Base`. `tally` / `Serve` hold a local of a field's name (an annotated local, an object
  literal's key, a `var` block) that is no field. Python's `Encoder` extends a class outside the
  project, so its `self.poster_id` is looked up by name and its nested `Options` is found as on
  master; `Point` declares `offset` first in a tuple. TypeScript's `Issue` binds `close` in its
  constructor, and `Issue.label` reads its fields inside `case …: {` blocks, which are no object
  literals, and inside the literal a `case …: return {` returns, which is one.

- `super` (#100): `supers` (Python and TypeScript; Go has no `super`) has an `Archive` whose
  `store` a `ColdArchive` and below it a `GlacierArchive` override, each calling `super`, a field
  and a method two levels up, `super` in a nested function (Python) and in an object literal's
  method (TypeScript), where it is not the class's, and in Python the cases of several bases: the
  first base declaring the member (`Mixed`), one base leading to it, a diamond whose `Right.flush`
  a walk by depth would pass (`Diamond`), and a first base outside the project (`Wire`).

Each step of #68 adds the cases it resolves to the tests over these files. A later step can
change what `d` shows on a line here, but the files stay the same shape in all three languages.
