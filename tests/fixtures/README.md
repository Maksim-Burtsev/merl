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
  project, so its `self.poster_id` is no other class's namesake (#342) and its nested `Options` is
  found as on master; `Point` declares `offset` first in a tuple. TypeScript's `Issue` binds `close` in its
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
  `var (` block, a list whose second name takes the value in its position), one declared below
  its use, one declared as two types under build tags, and locals that hide them; a `var` inside
  a function and a raw string's line declare nothing. `aliases` follows `type X = Y` through a
  second alias, another package and an alias declared in that package, next to a defined type
  with a method of its own. `platform` declares `Clock` once per platform (by file name and by
  `//go:build !windows`) and `Codec` under a tag that is no platform. `qualifiers` has an aliased
  import, a parameter of the alias's name, and `go-ledger`, whose path reads `ledger` while its
  package is `books` and `ledger` is a variable of `scopes`. `results` has a method with named
  results, a parameter and a local. After the review: `globals` also holds the locals the scope
  walk does not read (a header with a function-typed parameter, a receiver on one, a `var (`
  block in a function, a local handed on; the lines above a label since #330) and `globals_x_test.go`, the
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

- The comment marker is the kind's own: `//`, `#`, `--` or `;`, and OCaml's `(*`, whose closing
  `*)` at the end of the line is not part of the answer. Nothing but whitespace stands
  before it on the line, and nothing but spaces between it and the caret.
- The caret's byte column is the column in the probed line, so an annotation is indented as its
  line is (a tab under a tab in Go). The probed line is the nearest line above that is no
  annotation or `status:` line, so several annotations stack under one line.
- `d: FILE:LINE` is a jump there, `FILE` from the fixture's root.
- `d: picker FILE:LINE, FILE:LINE` is a picker holding exactly these rows, in any order; `, …` at
  the end means at least these.
- `d: none` is nothing found (the status line starts with `no `, or names the label the word is:
  `name: argument label`, `name: key`); `d: !jump` is anything but a jump.
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
`sql/`, `make/`, `terraform/`, `docker/`, `yaml/`, `graphql/`, `proto/`, `css/`). Each is one small shop (a `Tariff` and a `Coupon` sharing `rate` and
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

The kinds with no types keep the shop's names and bend its shape to what they have. Zig forbids a
name that shadows another, so `zig/` has two functions' locals of one name and imports of one name
in three files instead. `shell/` declares `describe` in two files, as two plugins would, and a
`local` named like a function. `sql/` has a CTE named like a table, a schema-qualified and a quoted
name, and a `$$` function body. `make/` declares a target in two `.mk` files and a double-colon rule
twice, and sets variables only by `+=` or for one target. `terraform/` has a module whose
`var.region` is its own, a local named like an attribute of a `tags` map, and a heredoc. `docker/`
and `yaml/` are searched file by file, so each file probes the stages or jobs of its own and one of
another file; `yaml/ci/` lists its GitLab stages in the block form beside jobs named after them, or
not at all (#473). Each of these kinds also has a file that opens with a glob or a lone backtick
above a declaration, and a heredoc or a block scalar holding a declaration-shaped line: today's
answers there are the known misses of #436, and the globs inside quotes (`"parcels/*"`,
`["src/**/*.rs"]`) guard what already works.

The adversarial corners of #931 for PHP, Lua and Bash keep a tea set's names too. `php/src/Tea/`
has a `Kettle` and a `Teapot` sharing `pour` over a `Vessel`, a namesake `Kettle` in
`Shop\Tea\Stale` imported under an alias, `use function` of a `steep` the file's own namespace
also declares, a function `pour` beside the methods, `static::` and `parent::`, a trait's method,
a parameter named like a field, a local assigned twice, a closure's `use`, and a heredoc and a
block comment declaring what the files declare; `Kettle::card` names its members in the prose
of a heredoc and a nowdoc, which declares nothing, beside interpolated `{$this->…}`, `$var` and
`${var}`, and an escaped `\$var` that is text; `Kettle::sign` does the same in one-line quoted strings, beside a callable `[$other, 'pour']` that is still looked up (#1113); `Kettle::ledger` looks up the code after a string that closes a line below its quote, and a backslash-qualified class name in prose. `php/src/Tea/Infuse/` is the third
corner (#1126), its answers read from the code: `Ceremony` calls `format` and `count` on a
`DateTimeImmutable` and an `ArrayObject` beside `Leaf`'s namesakes, a builder chain over lines, a
call on a call's result, `$cup = $cup->steep()`, a destructured pair, and one behind a
`...$spread`, which has no class, a loop variable assigned again, a class imported under an alias
by a group `use`, an inherited and an overridden method, a `: self` step on a subclass, which
continues with the class declaring the method, as intelephense reads `self` (#1186), in a chain and stored in a local or a first-class callable (`selfBound`), a `: static` result stored in a local, a trait's method
chosen by `insteadof` and one renamed by `as`, an interface's method, an anonymous class's
method, an arrow function's capture, a first-class callable, an enum's case and method, a
function declared inside `if (!function_exists(..))`, and a string (prose: `d` says none, #1113), a `#` comment and a heredoc
declaring what the corner declares. `lua/lua/tea/` has `local function pour` beside
`function M.pour` and `Kettle:pour`, a parameter named like a required module, `local n = n + 1`,
`local fact = function` against `local function fib`, `local later` assigned by `function
later()`, a loop variable hiding a local, and a long string and a block comment declaring names.
`lua/lua/shop/requires.lua` puts the cursor on `require` paths (#1061): modules of the project,
one whose last name is another directory's file, and one from outside. `lua/tea/spec.lua` binds
`local assert` and `local brew_global` at its top level, which the bare names of `lua/tea/cup.lua`
and `lua/tea/table.lua` must not reach (#1062), nor the key `assert = function` of a table
below a comment; its globals assigned in an `if` block and at the top level stay definitions (#1112).
`lua/tea/tray.lua` calls a bare `rinse` whose namesakes are `function S.rinse` in `saucer.lua` and
`spec.lua`'s `local rinse`, a picker of both.
`lua/tea/infusion.lua` is the third corner (#1126), its answers read from the code, over
`gaiwan.lua`: `string.format`, `table.insert` and `("cup"):upper()` beside the module's `format`,
`insert` and `Gaiwan:upper`, which stay the standard library's (none) unless the project patches
it (`function string.steeped`, `string.decanted = function`) or binds the name (a parameter `string`); `tostring(v):upper()`
still jumps (#1207); a field of a required module, `pcall(require, ..)`, a builder over lines and
through a local, whose `return self` is not followed (#1208), `local gaiwan = gaiwan.Gaiwan.new(1)`,
whose receiver is read on its own line, `M.Gaiwan = Gaiwan` answered by the module's local,
`local a, b = b, a`, a `for` header over a parameter of its name, a subclass made by
`setmetatable({}, { __index = .. })` with an inherited and an overridden method, `self` in a
closure and in a `function(self)`, a call above the `local function` of its name (a global, so
none), a one-line string, a `--` comment and a `--[==[` block declaring names, and `goto continue`
(#1209).
`shell/lib/tea.sh` and `shell/bin/tea.sh` have a variable and a function both named `pour`, used
as `$pour`, `${#pour}`, `(( pour++ ))` and as a command, `command steep`, case patterns named like functions, a
function declared twice, `for` and `read` variables, and two heredocs declaring functions.
`shell/bin/infuse` is a script with no extension, Bash by its `#!` line (#1056), and
`shell/NOTES`, with no `#!` line, holds a function-shaped line no lookup may find.
`shell/bin/brew.sh` assigns plain `name=` in a function twice, `+=` after `=`, a global above
the function, a heredoc line shaped like an assignment, and a variable named like a function
(#1057). `shell/test/shop.bats` loads `lib/warehouse.bash` and reads a variable another
`@test` assigns (#1058); `shell/lib/loader.sh` defines a `load` of its own and calls it beside
`lib/warehouse.bash`, so outside a `.bats` file, and on the word `load` in one, the call is
looked up. The end of `shell/bin/brew.sh` sources paths behind `$VAR/`, `${VAR}/` and
`$(dirname "$0")/`, a path a variable builds, the variable in front, and paths behind `$HOME`,
`${HOME}` and `~`, which share their tail with project files and must not open them (#1059).
`shell/bin/steeping.sh` and `shell/lib/infusion.sh` are the third corner (#1126), their answers
read from the code: the command `date` beside a variable `date=`, a function called in `$(..)`,
in a pipeline over lines and in a heredoc, `whisk=$(whisk 2)`, a `for` variable named like a
function, `read` in a `while` header, `read -ra`, `mapfile`, `printf -v`, `declare -n`, an
associative array, `${leaf:-..}`, a function declared in both arms of an `if`, a function named
in a `trap` string, a function declared in a function, `[[ -v ]]` and `unset`, `local leaf=$leaf`,
`export -f`, and a `: '..'` block and a quoted heredoc declaring functions.

Objective-C is a part of the C kind, so its cases are a corner of `c/` with names of their own:
`c/objc/` declares every form of #417 in `Repo.h` and `Repo.m` (a class, a protocol, a method by
each part of its selector, a property, a block property, the four `NS_ENUM` macros, a category, a
class extension, forward declarations, a declaration-shaped `/* */` block) and probes them all,
the refusals included, from `Profile.mm`.

The adversarial corners of #931 keep a tea set's names, so the shop's stay unique: a `Kettle` and
a `Teapot` sharing `Pour` / `decant`, a namesake `Kettle` in another package or namespace, a
local or parameter named like an import or a field, and declaration-shaped lines in a string and
a comment. `go/rival/` adds `:=` redeclaring `err` and a `var`, method values and expressions, an
`if` header and a closure re-declaring a local, a promoted method and one an embedding type
overrides, an interface method, a function-typed field and a generic method, and its `twin`
package imported plainly and under an alias. `go/orm/` (#1072) chains method calls off
`GetEngine()`, on one line and one call per line: through `Join`, which returns a type of a
package outside the project, a `Filter.Or` namesake must not be jumped to; through `Where`, the
project's `Query.And` wins over `Filter.And`. `csharp/src/Shop/Tea/` adds a partial class over
two files, `using static` beside a member of the same name, overloads, `base.`, an explicit
interface implementation, a lambda parameter hiding a local and a `using` alias of
`Shop.Tea.Stale`. `csharp/src/Shop/Pages/Till.razor` (#1088) is a Blazor component, read as a
class of its `@inject` fields, `@code` members and markup expressions: an injected `Kettle` that
only `_Imports.razor`'s `@using Shop.Tea.Stale` tells apart, a method named in an `@onclick`, and
an e-mail address, a text, a component tag, a `@formname` string and a Razor comment that name
nothing, nor do the declaration-shaped lines of markup after the `@code` block.
`c/include/tea/` and `c/src/tea/` add out-of-class definitions, a constructor's initializer list, overloads, ADL beside a global namesake, a qualified definition of
`stale::steep`, a `using` declaration, a namespace alias, a template and an `#if 0` block;
`caddy.cc` calls `scoop` from inside its namespace and from global code, each hiding the other,
and `caddy_use.cc` behind `using namespace caddy;`, where the picker stays (#944).
`leaf_alias.cc` passes an argument typed by a `using` alias named like `infuse::Leaf`, which
must neither pick an overload nor add a namespace. `dripper.hh` declares a pure virtual whose one
override is in `dripper.cc`'s other class: no prototype and definition of one function, so a
picker both ways (#1050).
`stale_again.cc` opens `namespace stale {` again and writes `using namespace stale;`, which
must not turn `stale::Kettle` into a picker, and has a `tray` that is a namespace in one place and
a class in another, where the picker stays (#1019); `wrap.cc` writes `tidy::Napkin` inside
`infuse`, where a `tidy` of its own and a global one are two namespaces, not one. `brew_line.cc`
(and `c/src/brew_line.c`) call a declared and defined function from a one-line function, whose
body holds no parameter (#1020); `brew.hh`'s prototype stays apart from the definition in
`brew_hidden.hh`, which the file does not include, and joins the one in `brew_impl.hh`, which it
does; an out-of-class `operator()` keeps its parameters.
`sieve.cc` reads the parameters of operators, in and out of their class, and a member of
the class inside an operator before a global namesake; a call inside a free operator in a namespace keeps its picker (#1029).
`infuser.cc` defines constructors inside their class, after `explicit`, over an initializer
list and a function-pointer parameter, and a destructor, each the answer on its own name, while
a construction with `new` inside a method and in a function still leads to the class (#1248).
`c/src/zl/` spells `static` as zlib does, `#define local static` in a header included through another
one, beside a macro for `extern` and one that is `static` in one branch of an `#ifdef` only, which
stay global (#1030).

The third corners of #1126 keep a tasting's names, `Cup`, `Mug` and `Stove` sharing `Sip` /
`sip`. `csharp/src/Shop/Tea/Infuse/` calls `StringBuilder` and `List<T>` members named like the
project's, a builder chained on one line and over lines, a call on a call's result and through a
`using` alias, `foreach`, `is Mug cup` and deconstruction binding a name a field also has, `cup =
cup.Refill()`, an inherited property, an interface default method, an extension method beside an
instance namesake, a local function, a primary constructor whose parameter a method's own
parameter hides, and declarations in a block comment, a raw interpolated string and `#if false`.
`c/include/tea/cup.hh` and `c/src/tea/tasting.cc` do the same in C++ with `std::string` and
`std::vector` members, a builder over lines, `->` on a call, a range `for`, `dynamic_cast` in an
`if`, a structured binding, a lambda's parameter, plain capture and init-capture each hiding a
local, `using` and `typedef` aliases, `using namespace` in a function and a macro.
`go/infuse/tasting.go`, its answers gopls's, calls `strings.Builder`, `bytes.Buffer`, `os.File`
and `time.Time` members named like `Cup`'s, a builder over lines and on one line, calls on
calls' results, `cup := cup.Warm()` in a block, a `range` and a three-clause `for` binding a
parameter's name, a type switch, a comma-ok assertion, a struct and an interface embedded two
levels deep, an aliased import beside its `Cup` namesake, a dot import, a type parameter's
constraint (one grouped with another; a union, an interface literal and `any` are not read, so no jump), a named result, and declarations in a raw string, a block comment and a struct tag.
`c/src/lab/swirl.c` (over `c/include/lab/carafe.h` through `lab.h`, and `c/src/lab/funnel.c`)
does the same in C, its answers clangd's: `struct tm`, `div_t` and `struct timeval` members named
like a project struct's fields, `->` on a call and over lines, a function-pointer field beside a
function of its name, an anonymous union, a `for` and a block each redeclaring a parameter, a
local named like its struct tag, a macro declaring a function, designated initializers, a
compound literal, a `struct` tag beside a function of its name, `goto` to a label named like a
local, and declarations in a string, a comment and `#if 0`.

`python/tea/` is the adversarial corner of #974 for Python, in a tea set's names: a `Kettle`
and a `Teapot` sharing `pour` over a `Vessel`, a namesake `Kettle` and `steep` in `tea.stale`,
a local hiding the imported `steep`, `global` and `nonlocal`, a comprehension variable named like
a module-level one, a class attribute beside an instance attribute of one name, `self.` and
`super()` in a subclass, a wrapping decorator, an import under `TYPE_CHECKING` and one in
`try`/`except ImportError`, a dataclass's fields, `match` captures (one named like an import),
two star imports of one name, and declaration-shaped lines in a docstring and an f-string. A
comprehension's iterable and a lambda's default read a module-level name. The `menu_*` modules
star-import `infuse` where the later-wins rule must give up (#980): a later module with
`__all__` (`listed`), one that mentions it without declaring it (`mentions`), an underscore
name, and a star import below the use or indented. `lamp.py` has a class attribute and a `def`
below it of one name, the `def` the answer: JavaScript's rule that a field shadows its method
(#1004) must stay out of Python.
`pouring.py` is the third corner (#1126), its answers pyright's, over `brewer.py` and `spout.py`:
members of a `str`, a file, `os.path` and a `dict` beside `Strainer`'s namesakes, a builder over
lines and chains of calls beside `Infuser`'s, a renaming re-export, an import of the module under
an alias and a class alias `Machine = Brewer`, `brewer = brewer.leaves(2)` and `cup = cup.next`, a
loop over `dict.items()`, a walrus, a class body's own attribute beside a comprehension and a
method that skip it, `this` for `self` over an abstract base, a nested `def` and an `except … as`
hiding module names, and an import in both branches of an `if`, which pyright answers with the
`else` one and the code with both. Its `brew` beside `brewing.py`'s: a module's own top-level
function wins over another module's that it does not import. Its `mixed` loops over sums of lists:
of `str` methods and literals (builtin), and with a `list[Brewer]` operand (not). Its `Lab` assigns `kit` through `cls` in a `@classmethod`, and `Rig` assigns `rig` from `cls.kit_class()`, a class attribute its subclasses override: `self.rig` is the `kit_class` nearest the class in its MRO, `Both` a diamond whose first base would pass the override in a walk by depth, and a `MakerRig` whose attribute holds a function, not a class; `SelfTrial` reaches it through `self.kit_class()` in a plain method (#1099). Its `clean` assigns
a `for` loop's own target, `line = line.strip()`, whose right side is the loop's (#1154), beside
a `while` inside a `for` of the same name, whose right side is either. Its `twice` calls a `str`
method on a local set from one set from a `str` parameter (builtin, #1151).

`markdown/` has no shop: Markdown declares nothing, and `d` there follows a link (#421).
`docs/notes.md` probes every form of link against the headings of `README.md` (two of one name,
a setext one, backticks and punctuation, an `<a id>`), code spans naming files (`mod.rs` is
carried by two), and links in a fence, a comment and the front matter. Its annotations start
with `#`, which Markdown reads as a heading: a `#^` right under a line is no heading but text, so
a reference definition below one needs a blank line to start its own block.

`graphql/` (#419) is a schema over two files and operations over two more: fields and enum values
beside selections, aliases and arguments of the same names, an `extend type`, a `"""` description
holding a type, fragment spreads and an `#import`.

`css/` (#415) has no shop: classes, ids, custom properties and Sass names are what it declares.
`src/styles.css` holds rules a class or an id of `src/Button.tsx` and `index.html` name, in the
selector shapes that style them and those that do not, a custom property with a theme's override,
`@keyframes`, `@property` and `@import`s; `scss/` `@use`s a module under an alias and under its
own name, `sass:math`, and a package of `node_modules/` (ignored, added with `git add -f`), and
composes classes with `&`; `less/` has a variable, a mixin called both ways and an `@import`.
Since #590 `src/Panel.tsx` takes classes from two CSS Modules, by a default and a named import,
each module declaring a `.container` the other must not answer for, and keeps TypeScript's answer
for a `styles` the import does not bind and for a plain `.css` import; `scss/legacy.scss` uses a
variable, a mixin and a function only `node_modules/` declares, beside a CSS built-in, a `sass:`
module's member and a placeholder it declares too, which it must not answer for; `sass/nav.sass`
writes indented rules with an `&` chain, a selector list and a comment that styles nothing,
which `index.html` names last.
Its annotations start with `//`, and with `#` in HTML, where they are text.

`proto/` lays its files out under a proto root, `proto/shop/v1/`, as buf does, with the well-known
types a project vendors under `third_party/`: its imports name paths from those roots, never from
the importing file. `shop.v1` and `billing.v1` each declare a `Money`, used unqualified and
qualified by each package, `.shop.v1.` absolute and `v1.` relative among them.

`cmake/` (#432) has a project's `CMakeLists.txt` over an `app/` it adds as a subdirectory and the
helpers, warnings and `Find` module it includes from `cmake/`: a function called in another case,
a target made through `${name}`, an alias and an imported target, a generator expression, each
refusal of the issue, and declaration-shaped lines in a bracket comment of each level, a bracket
argument and a quoted argument over lines.

`nix/` (#430) is a flake over a NixOS host that imports the shop's `lib/` through `../lib`: a
function by name, `let` bindings named `api` in two files (each stays in its own), a parameter
`pkgs` beside a `pkgs =` attribute of another file, a header over lines, a file and a directory
path, each refusal of the issue, and declaration-shaped lines in indented strings (one opened
after `//`, one holding the `'''`, `''$` and `''\` escapes), a `"…"`, a `/* */` and a `#` comment.

`haskell/` (#426) is a Cabal package whose `src/Shop/` modules import each other by module path,
one qualified (`C.weigh`), beside `Data.Map` from outside: a function with a signature and one
without, three equations of one function, two names on one signature, a class with a context and
its methods, an instance's methods, constructors on and under a `data` line, record fields, a
GADT, a type family, a pattern synonym, `go` in a `where` of two files (each stays in its own),
parameters, `do` and `let` bindings, case alternatives, each refusal of the issue, and
declaration-shaped lines in a nested `{- -}` comment and a quasi-quote.
`ocaml/` (#427) is a dune library: `money.ml` with its `money.mli`, a `cart.ml` calling
`Money.format_price` beside a local `prefix`, and `pricing.ml` declaring every form of the issue
(constructors on a `type` line, on `|` lines and in a GADT, record fields, `module type`, a
`sig … end = struct` module, a functor and its application, a class with a method), probed from
`basket.ml`: each refusal, a qualified call through two modules, a record field in a `with`
expression named like a function of the file, a local `acc` in two files (each stays in its own),
a polymorphic variant, declaration-shaped lines in a nested comment and a quoted string, `List.map`
beside a `let map` of the file, a module of the project that lacks the name, and in `receipt.ml` a
name of an `open`ed module used above the file's own namesake.
`fsharp/` (#427) is the issue's `Money.fs` and `Cart.fs` over a `Pricing.fs` in a namespace:
module lets behind their modifiers and an attribute, members of every kind, union cases, a record
over one line, an interface, a double-backticked name, `(*)`, and declaration-shaped lines in a
nested comment, a verbatim and a triple-quoted string, probed from `Courier.fs`.
`julia/` (#429) is a `Shop` package whose module includes `money.jl` and `tariff.jl`, with
scripts that `include` them by path: every declaration form of the issue (a `function`, one
`Base.show`, one-line methods with a return type and a `where`, `struct`, `mutable struct`,
`Base.@kwdef struct` with a default, `abstract type`, `primitive type`, `@enum`, `macro`,
`baremodule`, `const` and a column-zero global), `weigh!` beside `weigh`, a local `m` beside a
global `m` of another file and a local `total` beside the function `total`, `rate` both a method
and a field, a functor, each refusal, imports of a package that is not installed, and
declaration-shaped lines in a docstring, a nested `#= =#`, a backtick command over lines, a
regex and a raw string.
`r/` (#422) is a package, `shop` in its `DESCRIPTION`, whose `R/` holds every declaration form of
the issue (a function with `<-` and `<<-`, a lambda, a backticked operator and replacement
function, an S3 method, `.onLoad`, a top-level `=`), an S4 generic with two methods beside
`setClass` and `setRefClass`, an R6 class with its methods and a field, a list of handlers, a
`.r` file and a `.Rprofile`; a `dplyr::filter` beside the project's `filter` and a `shop::` call of
its own; a `source()` path; each refusal (a wrapped call with named arguments beside a top-level
`=`, field and element writes, a replacement call, a formula, a right assignment, a `for`
variable); and declaration-shaped lines in a `"…"` and a `'…'` over lines, a raw string with
parentheses and one with dashes and brackets, and a comment.
`perl/` (#424) is a `lib/Shop/` of three packages and a `bin/report.pl` and `t/order.t` that
use them: a package read whole from either part, a method behind its class (`Shop::Order->new`),
a qualified sub and variable, a call on a value by name, `my $self` in three files (each stays in
its own) and in two subs of one file, a sigil change (`$args{total}` to `%args`, `$#items` to
`@items`), a `for my` loop, `state`, a signature, a constant of each form, Moose's `has`, 5.38's
`class`, `method` and `field`, each refusal of the issue, a `new` the package does not declare
(offered, never jumped to), and declaration-shaped lines in a heredoc, an indented heredoc, a
`q{…}` over lines, POD and after `__END__`. Carton's `local/lib/perl5`, ignored by a `.gitignore`
of its own and added with `git add -f`, holds the `File::Basename` an import reaches outside the
project.
`gdscript/` (#434) is a Godot project whose `project.godot` autoloads `globals/game_state.gd` as
`GameState`: the issue's table, a `class_name` with its `extends` on the line, an inner `class`, a
signal from `emit()` and `emit_signal("…")`, members, named and anonymous `enum`s, a local
`direction` beside a member `direction` of `actor.gd`, a `for` variable, a parameter and a lambda's,
an autoload's function and the autoload itself, `res://` paths in `preload` and `extends`, each
refusal (a node path, a dictionary key, `extends CharacterBody2D`) and declaration-shaped lines in
a `"""` string.
`solidity/` (#433) is a token and a basket over a library: the issue's `ShopToken` with its
`modifier`, `event`, `error` and constant, imports of `@openzeppelin/contracts` from a
`node_modules` the fixture's `.gitignore` ignores (added with `git add -f`), a relative import
aliased and as a qualifier, a remapped one through `remappings.txt` and one through
`foundry.toml` into `lib/`, every declaration form of the issue, parameters, named returns and
locals of a header over lines, and declaration-shaped lines in a NatSpec `/** */`, a `/* */` and a
`//` comment.
`yaml/api/` (#435) is an OpenAPI description over two files and two JSON Schemas, for `d` on a
`$ref`: a pointer into the same file past a namesake key, one into another file (on each word of
the reference and on its punctuation), a whole file, an escaped path key, a quoted status code,
an item index, a `discriminator.mapping` value, an `$anchor`, a URL a file declares as its `$id`
and one none does, and the refusals: a `$ref` in a block scalar and in a comment, a flow-style
value and a missing key, which fall back to the YAML rules. A backtick in a description stands
above the anchor it must not hide. The `.json` files carry `//` annotations: the walk skips them
as JSONC comments, and the annotation test reads `.json` files though they have no kind.
`clojure/`, `emacs-lisp/`, `scheme/` and `common-lisp/` (#428) are a shop in each Lisp: every
defining form of the dialect, a namespace alias and a `:refer` (`money/format-price`, and
`str/join` of a namespace outside the project), a `let` binding named `total` beside a `(def total`
of another file, `empty?` beside `empty`, a `defmulti`, `cl-defgeneric` or `defgeneric` offered
with its methods, a `(comment …)` form, `#_`, `#;` and a nested `#| … |#`, a `(require …)` that
opens a file, each refusal of the issue, and declaration-shaped lines in a docstring.
`starlark/` (#431) is a Bazel module, `shop`, with two packages and two `.bzl` files: every
declaration form (a `def`, a rule, a provider, an aspect, a repository rule, a module extension, a
transition, a plain value), names a `load` takes (one under an alias, one over several lines, one a
parameter shadows), labels to a target in every form (`:api`, `api`, `//api`, `@shop//api:api`,
`@//api`), to a source file and to a repository (`@rules_go`, `@bazel_gazelle` by its
`repo_name`), each refusal of the issue (a rule call, a keyword argument, a computed name, a load's
strings, `native.genrule`, a `select` key), and declaration-shaped lines in a docstring.

`elixir/` ignores its `deps/` in a `.gitignore` of its own, as `mix new` writes it, and holds a
`deps/jason`, `deps/phoenix_live_view` and `deps/plug` added with `git add -f`: dependencies
the project walk does not reach, one whose module is no path and two declaring one name (#437). Its
Erlang (#425) is `src/shop_parcel.erl` over `include/shop_parcel.hrl` and `deps/ranch`: a module
called from `src/shop_depot.erl` and from Elixir's `lib/shop/report.ex`, records with a field on
the `-record` line and on the lines it wraps to, macros, types, clauses over lines, includes of
both kinds, each refusal of the issue, and declaration-shaped lines in a `"…"` and a `"""…"""`
beside a `%` comment holding `"""` and a `$"`. `lib/shop/seed.exs` calls a function in column zero,
which declares nothing. Its annotations start with `%`.

`typescript/tea/` is the adversarial corner of #974 for TypeScript, its answers tsserver's. A
`Blend` that is a type alias and a const, an interface `Vessel` and a namespace `Caddy` each
declared twice, an enum `Leaf` merged with a namespace, a function `steep` merged with one, a
default-exported `Kettle` imported under two names (one through a chain of `export *` and a
renaming re-export), `import type`, overloads of `boil`, destructured and renamed parameters
beside a module-level namesake, `this` in an arrow and in a `function` that declares `this:
Kettle`, generic constraints, optional chaining, and declaration-shaped lines in a JSDoc and a
template literal. `ladle.ts` holds where a constraint must not be read: the result of a
generic call, typed by its argument rather than the constraint, and a generic call taking a
callback, which declares no type parameter. `packaged.ts` types receivers by packages that are not
installed (`yjs`, `markdown-it`) beside project members of the same names, and calls a member
named like a top-level function and a class of the file (#1079); `cut.ts` reads a member named
like a local of another function. `migrate.ts` calls `changed()` on a `Revision` of `record.ts`,
whose base comes from the uninstalled `sequelize-typescript`, beside a type's `changed`, and reads
`isDevelopment` off the default export of `settings.ts`, typed `Record<…> & { … }` over several
lines, beside a class's getter of that name (#1079); `grown.ts` calls a member `Object.assign`
mixes into the prototype of a class whose bases reach that package, beside a namesake mixed into
another class. `crate.js` reads `hinge` off an untyped parameter, declared only
as a plain property of an interface in `node_modules/parcel-kit` (#1078). `rule.js` is an ESLint
rule whose visitors' `node` is the `@types/estree` interface the method names (#1078): an own
member, an inherited one, one the interface lacks (offered by name, as untyped), a nested
function's own `node` and a Babel-shaped `visitor` (both untyped), and a reassigned `node`, which
keeps its type as tsserver's does. A `CallExpression` visitor's `node` is the union alias
`SimpleCallExpression | NewExpression`: `callee`, which both declare from one base, jumps;
`optional`, declared by one, and `type`, declared by each apart, stay pickers. With
`@types/estree` installed, `rules/q.js`'s `callee` probes answer as tsserver does too.
`patterns.js` narrows a JSDoc-typed `Pattern` union by its `type` tag (#1078), as tsserver does:
inside `switch (pattern.type)`'s `case "ObjectPattern":` after a `return;`, and in the
consequent of `property.type === "Property" ? property.value`, each of a `for…of` element's
inline union `AssignmentProperty | RestElement`. A case fallen into, a switch on another
member, a negated ternary, a callback inside the case and the code after the switch stay pickers.
`infusion.ts` is the third corner (#1126), its answers tsserver's: members of a string, a
`Promise` and a `Map` beside project namesakes (`Basket.trim`, `Basket.then`), a builder chain
over lines through a renaming re-export (`Strainer as Filter`), an inherited and an overridden
member of `strainer.ts`, a loop variable, a destructured local and an arrow parameter hiding
namesakes, `basket = basket.next`, a nested function called above its declaration, a type behind
`export * as mesh`, a dynamic `import()`, a call inside a template literal holding a declaration,
a commented-out declaration, and `this` in a class expression. One answer is read from the code:
`sieve.sift` on `const sieve: Sieve = { sift: … }` is the literal's `sift`, where tsserver
answers the interface's. `caddy.ts` and `tins.ts` hold std names the project shadows: a project
`Array` whose `from` is no builtin, a project `ReadonlySet`, and an imported outside `ReadonlySet`
whose member is "not installed", never "builtin".

`typescript/coffee/` is the adversarial corner of #999 for JavaScript, in `.js`, `.mjs` and
`.cjs`, its answers tsserver's with `checkJs`. A constructor function `Grinder` with methods on
`Grinder.prototype` and `this` in an arrow and a `function` inside them, `module.exports =`,
`exports.darken = function` and `exports.Roaster = class`, a destructured and renamed `require`,
an anonymous `export default function`, a getter, a class field beside a method of its name in
another class, in the same one (also static, named like a prefix of `static`, beside a static
method read through `this`, declared after the method, named `get`, inherited by a subclass, and
behind `super`, where the answer is the runtime's method: tsserver 6 answers the field, its
binder dropping a method declared after a field of its name), a one-line getter above its
setter (tsserver offers both; `d` jumps to the setter, which is no field), a `var` declared
twice, a function called above its declaration, a JSDoc `@typedef`, and declaration-shaped lines
in a template literal and comments.
Two probes guard what a rule for `exports.name =` or `Ctor.prototype.name =` must not take: a
`require`d module named like one of its `exports` properties (`glazing.cjs`), and a default
import named like a prototype method of the exported constructor (`firing.mjs`).
`kettle.mjs` exports as its default an instance bound to a `const` (#1008). `sieve.cjs` assigns
to `exports.name` a value in parentheses, which declares nothing, an `async` arrow, an arrow with
a bare parameter and a named function behind `module.exports.`, read from `pouring.cjs` through a
destructured and a whole `require` (#1007). `mill.cjs` holds what a constructor function must
not declare (#1001): a function nested in its body, `Mill.grind` on the constructor itself and
`Grinder.burr`; beside a prototype member holding an arrow, an `async function` method, a class
extending `Grinder` and a JSDoc `{Grinder}` parameter. `scoop.js` declares a `var` again in a
`for`, in a nested function, in an arrow callback and in a template literal, and in another
function (#1003), in a promise chain's second callback, and in the next of two functions whose
bodies are not indented. `brew.cjs` names in JSDoc braces a `@typedef` of a string, a constructor
function and a class behind a `require`, and a `Kettle` the file does not import (#1002);
`dosing.js` holds braces JSDoc must not read: a code line continuing a product with `* f({ x })`,
an object literal after a one-line `/** @type {…} */`, and a word in braces in a plain comment.
`blooming.cjs` writes `exports.bloom = function` only in a block comment and a template literal,
then exports a named function behind `module.exports.bloom =`.
`hopper.cjs` is `module.exports = class Hopper extends Bin`, `Bin` itself `module.exports = class`
behind a `require` (#1076): `this.x` reads the class's own fields and the base's, and a namesake
field of `Roaster` stays out.
`ledger.js` declares several names in one `let` (#1077), without initializers and with a comma
inside one, beside a function `count` and another function's local `last` the later names must
not reach.
`pulling.mjs`, `steamer.cjs` and `wand.cjs` are the third corner (#1126), their answers
tsserver's with `checkJs`: the shapes of `tea/infusion.ts` in JavaScript over `lever.mjs` and its
re-exports in `espresso.mjs`, a JSDoc `import("…")` type, `const self = this` and `const that =
this` in a constructor function and a prototype method, and a name re-exported by a
`...require("./roast.cjs")` spread in `module.exports = { … }`.

`swift/Sources/Shop/Tea/` is the adversarial corner of #982 for Swift, its answers sourcekit-lsp's.
A `Brewing` protocol whose extension gives defaults, a `Kettle` that overrides one and a `Teapot`
that does not, members of `Kettle` declared in extensions, a nested `Kettle.Lid` beside a
top-level `Lid`, computed properties, `pour` overloaded by argument label beside a top-level
`pour`, two `init`s of `Cup`, `self` and implicit `self` in closures (`[weak self]`, `guard let
self`, `[self]`), shorthand `guard let` and a capture list rebinding a parameter, `if let` over a
property, a one-line closure whose parameter is named like a property, and declaration-shaped
lines in a multi-line and a raw string.
`swift/Sources/Shop/KeepLog.swift` calls free functions through a module `Package.swift` does not
name, only its folder `Sources/Kit/` (#1068): `Kit.record` jumps, `Kit.jot` offers its namesake
in `Diary` too, as the search by name does. `Larder.swift` returns an implicit `.x` from functions
returning `Self` (in a struct and in an extension of an outside type) and a generic `T`, which
the expected type must not read (#1069): each jumps by name, and says "no definition" with its
guard removed.
`swift/Sources/Shop/Infuse/` is the third corner (#1126), its answers sourcekit-lsp's: `Sencha`
declares `uppercased` and `append` beside a `String` and an `[Int]` local, a builder chain over
lines, members of a constructor's and a method's result, an override in the subclass `Gyokuro`,
a protocol extension's default beside a class's own, a class method reached through the subclass,
an extension of `StringProtocol`, `let sencha = sencha.steeped()`, a tuple in a `for` header and in
a `let`, `$0`, a `func` nested in another function beside a top-level namesake, a closure
parameter named like that function, `super` in an extension, a `typealias` in both arms of `#if`,
and a multi-line string and a block comment declaring what `Sencha.swift` declares. Where
sourcekit-lsp offers a non-final class's method with its override, the annotation takes the
declared type's method.
`swift/Sources/Shop/Faucets.swift` (#1072) calls a method with two overloads, both returning
`Faucet`, on a value an inherited method returns, and then `.pour` on the next line or the same
one: it lands on `Faucet.pour`, not `Cistern.pour`. Overloads that return two types (`either`)
keep the picker.

`rust/src/tea/` is the adversarial corner of #982 for Rust, its answers rust-analyzer's. A
`Kettle` with an inherent `pour` beside the `pour` of two traits, `Pour` and `Spill`, which
`Teapot` implements in two modules, called as a method, by `Trait::pour` and by `<T as Trait>::`;
`self.pour()` in a trait's default method and on a generic bound; `Self` as a type and a path;
`Kettle` and `Teapot as Pot` re-exported by `pub use`; an import shadowed by a local closure, a
local and a parameter; a struct and its method declared by a `macro_rules!`; a closure parameter
named like a field; an inherent `impl` in another module; and a doc comment and a raw string
declaring what `cups.rs` declares.
`rust/src/infusion.rs` (+ `infusion/leaves.rs`, `infusion/pantry.rs`) is the third corner (#1126),
its answers rust-analyzer's: std methods (`len`, `push`, `trim`) on `String`, `Vec` and `str`
values beside a project `Leaf` that declares them, a builder chain over lines, calls on a call's
result, a trait default overridden by `impl Steeper for Leaf`, a supertrait's default reached
through a bound, a trait implemented for `str`, `let leaf = leaf.trim()`, a loop header's tuple
pattern and a struct pattern, a glob import beside the file's own `fn gulp`, a function nested in
another, a closure parameter named like a function, an alias re-exported through `self::pantry`
(`Leaf as Sheet`), a `type Tin = Leaf`, a `{gulp}` inside `format!`, and a string and a block
comment declaring `gulp` and `finish`. `kind()` is imported under `#[cfg(unix)]` and
`#[cfg(not(unix))]` from two modules: rust-analyzer answers the host's arm, merl offers both.
Since #1044 `shadow_back_to_leaf` rebinds `leaf` to a `Leaf` through `unwrap`, which keeps its
jump, and `label_of` holds the `String` a `Leaf` method returns, which `Leaf::len` never answers.
`brew_note` binds `ref note` in a `Brew::Steeped { .. }` pattern and `steeped_notes` loops `for
note in &notes` over a `Vec<String>`: both hold a `String`, which `Leaf::len` never answers either.
`poured_note` binds `ref note` in an or-pattern of two `Pour` variants whose `note` is a `String`
in each, and `drip_note` binds `ref note` at the `String` place of the tuple variant `Drip::Slow`;
master jumped to `Leaf::len` on both. `kettle_note` binds `ref note` in `Kettle::Hot { .. }`, a
variant written on one line, where master jumped to `Leaf::len` too. `Canister` is a tuple struct:
`self.0` holds `leaves::Kettle`, a project alias of `Vec<u32>` named like the enum `Kettle`, so
its `len` is `Vec`'s, where master jumped to `Leaf::len`, and `self.1` holds a `Leaf`.
`poured_split` is `poured_note`'s or-pattern split over two lines, the second led by `|`, where
master jumped to `Leaf::len`.

`sql/migrations/` and `sql/numbered/` are diesel-style migration folders for #1064: a
`strainers` table created, re-created by a later `up.sql` and again by its `down.sql`, a
function and a trigger sharing a name, the same table read from a plain file outside any
migration, and `1_init` beside `10_more`, whose order is unclear and keeps the picker.
`pour_strainers` reads columns for #1063 through an alias and a table: one from a `CREATE TABLE`
body, one a quoted name, one an `ADD COLUMN`, one a `RENAME COLUMN … TO`, and none through a
derived table, a CTE named like a table, or an unknown alias.
`audit_ladles` creates an `audit.ladles` after `ladles`: a column through `FROM ladles l` stays
in `ladles`, one only `audit.ladles` has answers nothing, a bare one in `FROM audit.ladles` is
`audit.ladles`'s, and `ladles` itself offers both.
`touch_ladles` and `touch_strainers` create a trigger `touch` on two tables and a function
`scoop` with two signatures, one of them re-created: the two triggers and the two signatures
in effect stay a picker. `fill_ladles` reads bare columns
where SQL names one table: an `INSERT` column list, both sides of an `UPDATE`'s `SET`, a
one-table `SELECT` and its subquery, `ALTER TABLE` (`RENAME`, `DROP`, a `CHECK`), a trigger's
`UPDATE OF`, and `ADD COLUMN`s on the lines under their `ALTER TABLE`; and none where a second
table, a derived table, a function in `FROM`, an `AS` of the same name or a `REFERENCES` could
own the name.

`sql/tea/` and `dart/lib/tea/` are the adversarial corners of #931 for SQL and Dart, in a tea
set's names. `sql/tea/` has a `kettles` in two schemas, a CTE with a column list and one
`AS MATERIALIZED` named like a table, a CTE inside a subquery whose name the table keeps after
it, an alias named like a table, an overloaded function, a view declared twice, a parameter
named like a table, a statement in a string over lines and in a comment, and tables after a
`'C:\\'` and an `E'it\'s'`. `dart/lib/tea/` has a
`Kettle` with named, redirecting and factory constructors and a `part`, a namesake `Kettle`
behind an import prefix, a `Teapot` sharing `pour` and a static `cold`, a parameter hiding the
prefix and one hiding a field, `super.` and a cascade, record and object patterns, a local
function, and declaration-shaped lines in a raw string, a nested `/* */` and a `///` comment.
`dart/lib/infuser.dart` (#1072) reaches `Leaf.unfurl` through a call and a getter of `Infuser`,
over lines and on one, beside a namesake `Bag.unfurl`; a generic method's `T` is no type to
follow, so `hold(Bag()).unfurl` does not jump.
`dart/lib/brew/ceremony.dart` is the third corner (#1126), its answers read from the code, over
`chawan.dart`, `old_chawan.dart` (a namesake `Chawan` behind a prefix) and `lib/brew.dart`, which
re-exports: `String.trim` and `StringBuffer.write` beside `Chawan`'s namesakes, which say none with
no SDK, while an `extension .. on String`, `on Iterable<T>` or `on Object` member is still found (one `on Duration` is not); `const Duration(..).inMinutes`
(#1218), a chain in a local's value and a top-level function's result (#1218), a chain over lines,
a cascade over lines (#1072), a loop variable named like a later local, a list pattern (#1219), an
object pattern in an `if case`, a mixin's, an interface's, an inherited and an overridden method,
named and unnamed extensions, a prefixed named constructor, a class through a re-exporting prefix
(#1221), `${..}` in a string beside a `'''` string and a `//` comment declaring names, a local
function, a class method that hides the top-level function of its name, an enum's value, field and
method, `call()` on a field, and a type parameter's bound (#1220).

`sql/steep/` is SQL's third corner (#1126): a `urns` that `INHERITS` a `samovars` and a
`cozies` made `LIKE` it (#1222), a view's column, output aliases with and without `AS` in an
`ORDER BY` (one without `AS` named like the table's `id` must not jump to it, nor one after a
`CASE … END`, while `DISTINCT spout` still does), `FROM ONLY` and `TABLESAMPLE`, a column alias
list, an `INSERT … ON CONFLICT … DO UPDATE SET … EXCLUDED. … RETURNING`, `CREATE INDEX ON` and
`COMMENT ON COLUMN` (#1223), a CTE's column and a `WINDOW` name (none, as #1063's decision A
says; #1228 asks whether they should jump), a `"Lid"` beside a `lid` (#1224), a `LANGUAGE sql`
parameter named like a column (#1225), and `CREATE`s in a `$$` string inside a `$body$` (#1226)
and inside a nested block comment (#1227). `steep/grades.sql` names a type like a column of its
table: after `::`, after `TYPE` and as an added column's type the name is the type, and in a
`CAST(.. AS ..)` too, while the column itself still jumps to the column. The end of
`steep/plpgsql.sql` holds `SET` field targets and `MERGE` lists outside plpgsql, a `tins` with a
column `tins` whose name before `SET tins.seal` is the table, and an UPDATE alias a subquery or a
CTE's outer query binds again to `urns`.

`jvm/src/main/java/tea/` and `jvm/src/main/kotlin/brew/` are the adversarial corners of #961 for
Java and Kotlin, each in names of its own so the two do not answer for each other. `tea/` has a
`Kettle` whose field, constructor parameter and local share a name with a field of its base
`Vessel`, `this.` and `super.`, a static import beside a member of the same name, overloads by
arity and by type, a lambda parameter hiding a field, a nested `Lid` beside the package's `Lid`,
a `Cart` whose nested `Tray` with two constructors hides the package's `Tray` (#965),
a namesake `Kettle` in `tea.stale` imported over the package's own, and declaration-shaped lines
in a Javadoc and a text block. `brew/` has a `Samovar` whose property is initialized from a
constructor parameter of its name, an explicitly imported object function beside a member, a
nested and a top-level `Cosy`, a companion object, `it`, a lambda parameter and `this@Samovar`,
extension functions (one named like a member) with their implicit and labelled `this`, named and
default arguments, `when` subjects and smart casts, an import alias of `brew.stale.Samovar`, and
declaration-shaped lines in a KDoc, a raw string and a string template.

`jvm/src/main/java/getters/` holds chains of getters (#1071): an `Account` whose Lombok `spec`
and explicit `getMetadata()` lead to a `@Data` `Spec`, an interface `Meta` and a `@Getter`
field of `Owner`, beside a `Sender` declaring the same getters, a `getDisplayName()` that returns
a type outside the project, a chain off a call that is no getter (a picker of the explicit
getter and the Lombok fields) and a getter returning a type parameter. `jvm/src/main/java/typeargs/`
calls `Specs.<Build, Map<String, ?>>single(..)` (#1074), whose declaration nests its type
parameters, beside an `Other.single` that a read by name would offer. `jvm/src/main/java/typeuse/`
(#1115) calls static methods of project types: one returning `@Nullable T`, one behind an
annotation with arguments the rules cannot read (a `Ledger` namesake must not be jumped to),
and a `Specs.single` named like the one of `typeargs`, in another package. `jvm/src/main/kotlin/ladle/`
calls through a type's name the extensions declared in another file: on its `Companion`
(`Ladle.dollop()`, `Ladle.brim`), on an `object`, and on an `object` written with its package
(`fun ladle.Spoon.scoop()`), which only the extension test of the owner's hits keeps a jump.
`jvm/src/main/java/arity/`
calls methods on receivers `d` cannot type (#1075): one no namesake's parameters can take (all
are offered), two that fit, one of two by count, a varargs one, one on the line that closes a
text block, and a Kotlin extension called through its file class (`LoudKt.holler`);
`jvm/src/main/kotlin/named/` has two `Knob` overloads told apart by a named argument.

`jvm/src/main/java/quern/` and `jvm/src/main/kotlin/press/` are the third adversarial corners
(#1126) for Java and Kotlin, their answers read from the code. `quern/` has a `Quern` extending
`Stone` and implementing `Turnable`, whose default `crank` it calls bare and through
`Turnable.super`, a `Hopper` in its own package beside one behind `import quern.parts.*`, a static
wildcard import whose `notch()` a parameter `notch` must not hide, `StringBuilder.append` and
`String.strip` beside a project `Spout` declaring both, a builder chain over lines, a local named
like a field and a method (`Hopper hopper = hopper()`), a loop variable hiding a field, type and
record patterns in `instanceof` and `case`, an anonymous class, an inner class and
`Quern.this`. `press/` has a `Cafetiere` extending `Carafe` and implementing `Lever`, a member
beside an extension of its name, a `String` local beside a project `trimIndent`, an extension on
`String`, a builder chain over lines, `with`, `let`, `apply` and `also`, `val plunge = plunge(2)`
and `var plunger = plunger`, destructuring in a `val` and a loop header, a star import beside a
same-package `Mesh`, an alias import of a function, a local function hiding a member inside an
object expression, and `this@Cafetiere` from an inner class.

`ruby/lib/tea/` is the adversarial corner of #982 for Ruby, its answers read from the code (no
Ruby here runs ruby-lsp). A `Kettle` opened in three files, a `Vessel` base with an
`attr_accessor` beside a local of its name, a `Pourable` mixin whose `pour` a `Teapot` overrides
and a `Fillable` one the `Kettle` extends, `def self.cold` beside an instance `cold`, `class <<
self`, `define_method(:whistle)`, `steep = steep() + 1` and `steep * steep()`, a block parameter
hiding a method on its own line and one hiding a parameter over lines, and a heredoc declaring
`descale`, `Teapot` and `whistle`.
`pouring.rb` is the third corner (#1126), its answers read from the code, over `samovar.rb`:
core methods of a `String` and a `Hash` beside `Strainer`'s namesakes, a refinement of `String`,
a builder over lines and a chain on one line beside `Infuser`'s, `cup = cup.next_cup`, `strength
= strength.to_i` beside a method `strength`, a block-local `|c; strength|`, `alias_method`, a
constant alias `Urn = Samovar`, `tap`, `&.`, `::Tea::Cup`, a `Struct` member, `module_function`,
`super` past an included module, interpolation in a string and a heredoc declaring `quick`, and a
class body's local read by `define_method`. Its `Guess` keeps the guesses out (#1159–#1163): a
parameter reassigned under `if`, a local assigned twice, a block's value before `.dig`, and
`super` inside `define_method`, each answered as master does. `glaze.rb` patches `String` five
ways (`String.include`, `String.prepend`, `String.class_eval`, `refine(String)` and a reopened
`class String`): each patched core method is the project's, never "builtin", while a `chop` in an
unrelated class leaves `raw.to_s.chop` builtin. `new` lands on the `initialize` Ruby calls
(#1185): `Samovar` and its alias `Urn` reach `Vessel`'s, `flask.rb`'s `Flask` an included
module's and `Carafe` its grandparent's, while `Brewer` (none anywhere), `Thermos` (its own
`def self.new`) and `Jug` (a base outside the project) answer nothing.

### Adding a kind

1. Make `tests/fixtures/<kind>/`, a small project laid out as the language lays one out (a
   `Cargo.toml`, a `go.mod`, `src/main/java/…`), with the shop's shape above and at least 20
   annotations.
2. Write each answer as you know it should be. Run `cargo test annotation`; for a failure,
   either the annotation is wrong (fix it) or `d` is: then write today's answer with
   `; want <right answer> (#N)`, `#N` the sub-issue of #305 that covers it, or a new issue under
   #305 when none does.
3. No Rust code: the test finds the directory by itself, and `fixture_app` leaves every kind
   without a standard library or dependencies, so nothing outside the fixture answers but the
   typeshed stubs merl carries for Python (#1100), written `stdlib/logging/__init__.pyi:140`.
