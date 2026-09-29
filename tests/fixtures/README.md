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
  (`make_audit` / `makeAudit`, whose only `return` constructs an `AuditLog`), and through
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
  a walk by depth would pass (`Diamond`), and a first base outside the project (`Wire`). The
  `Tank` classes repeat the diamond and the outside base one level under the direct base, and
  add two bases leading to one declaration and an `ABC`; TypeScript's `ColdVault.open` narrows
  the return type of the `open` it calls through `super`.
- scopes (#100): `scopes` has a module-level (Go: package-level) `ledger` of one type and
  functions that declare their own of another: as a local, in a block inside a function that has
  one too, as an arrow function's parameter, read from a closure; one that declares none and reads
  the outer one; one whose inner `ledger` has no readable type; and the ambiguous ones: two
  branches of an `if` (Python), an `if` header and its body (Go), an arrow function's parameter on
  the cursor's line or on the line its statement started on (TypeScript). Python's `save` is a
  parameter named like a module-level `def`. `sibling` / `ScopedSibling` hold what must not leak:
  a callback and a loop in the `if` branch of an `else` (`try` of a `catch`), a callback on the
  line of an `if` and in front of a chained `.forEach(`; `documented` has an assignment in its
  docstring; `wrapped` an arrow whose line ends in `=>`.
- element types (#100): `elements` loops over collections whose type is written — an annotated
  parameter (`list[T]`, a quoted `tuple[T, ...]`, `T[]`, `ReadonlyArray<T>`, `[]*T`, `map[K]T`),
  the declared return type of `load_repos`, Go's `make`, a slice literal and a named `RepoList` — and over what hands
  out something else: a `dict`'s keys, a `Map`'s pairs, `for … in`, `enumerate`, a channel, a
  list assigned again with no type, two annotations that disagree. Each also calls a member on
  the collection itself. Since then the unknown receivers of Go's `Forget` and `Show` come from a
  channel, not a slice.
- calls (#100): `calls` has a `Depot` whose `people_repo` declares what it returns and whose
  `trail` constructs it in every `return` (Go: the first of two results), assigned to locals from
  a typed parameter and from one with no type; a `Source` interface whose method line declares
  the return type (TypeScript and Go); chains that hang off `open_depot()`, off the class itself
  and off Go's `store.Open()`; and what stays by name: a call of a call, Python returns that
  differ, a `return None` among them, a decorated function (the decorator on one line and over
  several), TypeScript overloads, a parameter named like the module's `open_depot`, two locals
  assigned from each other. `Yard` calls an inherited method and one on a field. Since then
  `make_uow().users.delete_user` in `chains` and Python's `make_audit()` in `factories` are
  proven.
- casts (#100): `casts` casts an untyped `found` to `UserRepository` and `AuditLog` in every
  form — `cast(T, x)`, `typing.cast("T", x)`, `x as T`, `x as unknown as T`, `v, ok := i.(T)`,
  `v := i.(T)` — assigned to a name and with the member hanging off the cast, and to what the
  project does not declare (`"Missing"`, `typing.Any`, `any`, `Partial<…>`, `fmt.Stringer`).
  `casts_own.py` declares a `cast` of its own, which is a function and no cast. Go has
  two type switches over a `v`, one after the other: a `case` of one type, from a block inside the
  arm too, a `case` of two types and `default`.
- TypeScript as it is written (#100, #131), in files of their own with no Python or Go twin:
  `headers` has classes whose header prettier wrapped — type parameters ending in
  `> extends Crate<K> {`, one of them with an arrow in it, the clauses over a lone `{` — a `Bin`
  whose type parameter is only constrained by `Sealable`, a function behind a wrapped `<…>` and a
  block of its own under a statement; `fluent` breaks member accesses in front of their dots, with
  a comment between two links, one at the end of the line above, a call of a call and a call
  closed on its own line; `optional` has `!.` and `?.` in every position; `privates` a `#addRoute`
  beside an `addRoute` and a subclass with a `#addRoute` of its own; `nest` a NestJS service with
  a wrapped, decorated constructor (hence `experimentalDecorators`), `const { repo } = this` in
  its forms, and classes behind namespaces of `nest_parts` and of the file itself, each with a
  namesake outside the namespace. `aliased` declares a `Trunk` it exports as `TrunkBase`, beside a `Trunk` inside a namespace and a
  re-export under a new name, and `aliased_use` extends and types by it. `shadowed` has a namespace `svc` and a
  function whose parameter `svc` is something else. `scopes` ends with destructurings wrapped over
  several lines: with a name commented out at the margin, behind a default, and a plain `const`
  closed by `} as T;`.

- Go (#100): `globals` reads package-level names that `globals_vars.go` declares (a `var`, a
  `var (` block, a list whose second name has no value the rules pair up), one declared below
  its use, one declared as two types under build tags, and locals that hide them; a `var` inside
  a function and a raw string's line declare nothing. `aliases` follows `type X = Y` through a
  second alias, another package and an alias declared in that package, next to a defined type
  with a method of its own. `platform` declares `Clock` once per platform (by file name and by
  `//go:build !windows`) and `Codec` under a tag that is no platform. `qualifiers` has an aliased
  import, a parameter of the alias's name, and `go-ledger`, whose path reads `ledger` while its
  package is `books` and `ledger` is a variable of `scopes`. `results` has a method with named
  results, a parameter and a local. After the review: `globals` also holds the locals the scope
  walk does not read (a header with a function-typed parameter, a receiver on one, a `var (`
  block in a function, the lines above a label, a local handed on) and `globals_x_test.go`, the
  external test package importing under a variable's name; `platform` a `Timer` whose method is
  per platform, a `Gauge` under `windows && !slow`, a `Gate` no CI host builds and a `Meter`
  under a tag of its own, the last two used from `platforms_gate.go` (`//go:build gated`);
  `aliases` an alias with a method of its own over an embedded namesake; `qualifiers` a parameter
  behind a function-typed one and a field called like the import.

Each step of #68 adds the cases it resolves to the tests over these files. A later step can
change what `d` shows on a line here, but the files stay the same shape in all three languages.

## Annotated cases (#307)

Besides the `navigate_*.rs` tests above, every fixture carries cases whose answer is written in
the fixture itself: a line comment under the probed line, with a caret under the word.

```go
return Order{Address: addr}
//           ^ d: shop/order.go:12
```

One test, `d_answers_every_annotation_in_the_fixtures` (`src/app/tests/annotated.rs`), walks
every directory of `tests/fixtures` as a project, finds every annotation, puts the cursor on the
caret's column of the line above, presses `d` and compares. It reports every failure at once, as
`fixture/file:line: want …, got …` with the status line, the answer written in the grammar below
so it can be pasted. `cargo test annotation` runs it and the grammar's own test; it takes a
couple of seconds.

The grammar:

- The comment marker is the kind's own: `//`, `#`, `--` or `;`. Nothing but whitespace stands
  before it on the line, and nothing but spaces between it and the caret.
- The caret's byte column is the column in the probed line, so an annotation is indented as its
  line is (a tab under a tab in Go). The probed line is the nearest line above that is no
  annotation or `status:` line, so several annotations stack under one line.
- `d: FILE:LINE` is a jump there, `FILE` from the fixture's root.
- `d: picker FILE:LINE, FILE:LINE` is a picker holding exactly these rows, in any order; `, …` at
  the end means at least these.
- `d: none` is nothing found (the status line starts with `no `); `d: !jump` is anything but a
  jump.
- `status: TEXT` on the line right under an annotation is optional: the status line contains TEXT.
- A known miss records today's answer and the wanted one: `d: none; want shop/order.go:12 (#NNN)`.
  Only the part before `; want` is checked, so the suite stays green; the wanted answer must
  parse and name its issue. The fix of #NNN flips the annotation to the wanted answer in its own
  PR.

Line numbers count the annotation lines too, so a line added in the middle of a file moves every
target below it: add cases at the end of a file, or in a new file.

### Where the cases live

The Python, TypeScript and Go projects above keep their lines, since the `navigate_*.rs` tests
pin them, so their annotated cases are a package of their own inside them: `python/shop/`,
`typescript/shop/`, `go/shop/` and `go/cart/`. Every other kind has a directory of its own
(`rust/`, `jvm/`, `ruby/`, `c/`, `csharp/`, `swift/`, `php/`, `lua/`, `elixir/`, `zig/`, `shell/`,
`sql/`, `make/`, `terraform/`, `docker/`, `yaml/`, `graphql/`). Each is one small shop (a `Tariff` and a `Coupon` sharing `rate` and
`describe`, a `Courier`, `discount`, `weigh`, a basket that uses them) holding:

- two types sharing a method name, an import inside the project (aliased, of a module, of a
  package that hands the name on), a parameter that shadows an import and a local that hides a
  module-level name;
- a declaration-shaped line in every multi-line literal the kind has: a docstring, a raw or
  multi-line string, a template literal, a text block, a block comment;
- every declaration form of the kind's row of `docs/navigation.md`, each probed once;
- the known misses of #305's sub-issues for the language, as `; want … (#N)`.

The shop's names (`Tariff`, `Coupon`, `Courier`, `Basket`, `gross`, `weigh`, …) appear nowhere
else in a fixture: a namesake would change what the `navigate_*.rs` tests find by name (a `total`
method did).

The kinds with no types keep the shop's names and bend its shape to what they have. Zig forbids
a name that shadows another, so `zig/` has two functions' locals of one name and imports of one
name in three files instead. `shell/` declares `describe` in two files, as two plugins would, and
a `local` named like a function. `sql/` has a CTE named like a table, a schema-qualified and a
quoted name, and a `$$` function body. `make/` declares a target in two `.mk` files and a
double-colon rule twice. `terraform/` has a module whose `var.region` is its own, a local named
like an attribute of a `tags` map, and a heredoc. `docker/` and `yaml/` are searched file by file,
so each file probes the stages or jobs of its own and one of another file. Each of these kinds
also has a file that opens with a glob or a lone backtick above a declaration, and a heredoc or a
block scalar holding a declaration-shaped line: today's answers there are the known misses of
#436, and the globs inside quotes (`"parcels/*"`, `["src/**/*.rs"]`) guard what already works.

`graphql/` (#419) is a schema over two files and operations over two more: fields and enum values
beside selections, aliases and arguments of the same names, an `extend type`, a `"""` description
holding a type, fragment spreads and an `#import`.

`elixir/` ignores its `deps/` in a `.gitignore` of its own, as `mix new` writes it, and holds a
`deps/jason` added with `git add -f`: a dependency the project walk does not reach (#437).

### Adding a kind

1. Make `tests/fixtures/<kind>/`, a small project laid out as the language lays one out (a
   `Cargo.toml`, a `go.mod`, `src/main/java/…`), with the shop's shape above and at least 20
   annotations.
2. Write each answer as you know it should be. Run `cargo test annotation`; for a failure,
   either the annotation is wrong (fix it) or `d` is: then write today's answer with
   `; want <right answer> (#N)`, `#N` the sub-issue of #305 that covers it, or a new issue under
   #305 when none does.
3. No Rust code: the test finds the directory by itself, and `fixture_app` leaves every kind
   without a standard library or dependencies, so nothing outside the fixture answers.
