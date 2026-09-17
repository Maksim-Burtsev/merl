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

Each step of #68 adds the cases it resolves to the tests over these files. A later step can
change what `d` shows on a line here, but the files stay the same shape in all three languages.
