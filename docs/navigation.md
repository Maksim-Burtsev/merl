# How navigation works

The rules behind `d`, `u`, `D` and `s`, in full. The [README](../README.md#languages) has the
short table of languages; the [full one](#what-d-recognises-language-by-language) is below.

## `d`: go to definition

There is no language server and no index: every lookup is a regex over the project's files,
run through [ripgrep](https://github.com/BurntSushi/ripgrep)'s library crates. `d` knows the
declaration forms in the [table below](#what-d-recognises-language-by-language) and searches only
where such a definition can live.

A word an import brings in, or the module in front of it, is looked for in the module the import
names. When that is the project's own code, only its file or package is searched, so a
same-named class in a test fake or another package is not a candidate. Python's
`from app.repos import UserRepo`, `import app.repos as r` and `from .repos import UserRepo` read
`app/repos.py` or `app/repos/__init__.py`, at the root, under `src/` or deeper, but not inside
another package. TypeScript's `./x` reads `x.ts`, `x.tsx`, `x.d.ts`, the JavaScript forms or
`x/index.*` (`./x.js` finds `x.ts` too), and an alias such as `@/x` goes through the `paths` and
`baseUrl` of the nearest `tsconfig.json` and the configs it extends. A named import finds that
name, aliased or not; a default import finds the declaration under its local name, or else the
module's `export default`; `ns.x` behind `import * as ns` finds `x`. A Go import path below the
`module` of a `go.mod` in the project is that package's directory. The word must be declared
directly in the module, or in the class the chain goes through (`UserRepo.create`), so `store.Open`
never lands on a method called `Open`. The status line names the file or the package:
`UserRepo: via import app/repos.py`, `Open: via import store/`. A Python module of the project that
does not declare the word but imports it — a package's `__init__.py` — hands it on: its own
module-level imports are followed, under another name and through `from .labels import *` too,
four modules deep, and the status line names the file the word ends up in. Two sources, a name
the module also assigns, an import inside a function and a cycle are not followed. Any other
module that does not declare the word itself — an `index.ts` that re-exports it — is not followed
further: `d` falls back to the search by name below and says `by name`. Behind
`from repos import UserRepository as Users` a receiver typed `Users` is a `UserRepository`.

An import of anything else is looked for outside the project first, even when the project declares
a word of the same name (Rust still looks in the project first). A word no import binds goes
outside only when the project has no definition of it. Outside means the standard library and the
installed dependencies the toolchain on this machine knows about — `sys.path` of
`.venv/bin/python` (or `python3`), `rustc --print sysroot` and the crates in `Cargo.lock`, `GOROOT`
and the modules in `go.mod`, the `node_modules` of every directory from the open file up to the
project root (a workspace keeps a package's dependencies beside it) — narrowed to the module the file's imports bind the
word to: `np.array` behind `import numpy as np` looks in `numpy`, `load` behind
`from json import load` in `json`, `Regex::new` behind `use regex::Regex` in the `regex` crate,
`chromium.launch()` behind `import { chromium } from 'playwright'` in that package; a bare
`std::fs::read_to_string` is its own path, and a relative import (`from . import views`,
`./utils`) is never looked for outside. A Go package is one directory, so `sql.Open` behind
`import "database/sql"` reads the files of `database/sql` and none of `database/sql/driver`, the
standard library's right under GOROOT's `src`; a package that is not installed is a search by
name, never `via import` of the directory above it. Of a TypeScript package installed more than once, `d`
reads the copy Node loads: the nearest `node_modules` that has the package or its `@types`, and a
farther one when the nearer lacks the imported path (`lib/extra`), unless the nearer maps its paths
with `exports`, which is then looked for as before; a pnpm link is followed to its version,
whatever the store directory is called, and a copy of JavaScript alone is read with the nearest
declarations further up. A name the copy's entry (as its `package.json` names it) exports
under another (`export { parseCookie as parse }`) is followed to its declaration; one only another
copy declares is found by name. A module of Node's own (`buffer`, `node:util`) is `@types/node`'s,
whatever npm package has the name, and never a project file. A workspace package linked into
`node_modules`, and an alias such as `@/lib`, `~/lib` or `#lib`, is the project's own: its source is
searched by name first, then outside, by name. A compiled module such as `orjson` lands in its `.pyi`
stub. The standard library's hits come before the dependencies', the picker shows paths relative
to their root, and files opened from there are read-only. Go's `_test.go` files, `testdata` and
nested modules such as GOROOT's `cmd` are skipped, since no import reaches them. C and C++ have no
per-project manifest — what the build system was told with `-I` is not in the source — so their
roots are the system headers: the SDK `xcrun` reports on a Mac, `/usr/include` on Linux, and
`/usr/local/include` and `/opt/homebrew/include`. `#include` binds no name of its own, so nothing
narrows the search — not even a `std::` qualifier, which names a namespace and no directory — and a
word the project does not declare is looked for in all of them. The C++ standard headers carry no
extension, so `<vector>` itself is not read; what its implementation puts in `.h` files is. C# has
nothing to point at — a NuGet package ships compiled assemblies and the runtime's own source is not
on the machine. Swift has what SwiftPM checks a package's dependencies out into,
`.build/checkouts`; its standard library ships compiled, with no `.swift` file to read, and an
`import` names a module and makes everything in it visible unqualified, so it binds no name of its
own and nothing narrows the search. PHP has Composer's `vendor/`, which is gitignored and so
outside the project walk the way `node_modules` is, and a `use Illuminate\Support\Str` in column
zero binds `Str` to that path, since PSR-4 spells a namespace the way the file system does; an
indented `use` pulls in a trait and names no file. Zig's root is the `std_dir` its own `zig env`
reports; its dependencies live in the global package cache under hashed directory names no source
line spells out, so they are left out, and `const std = @import("std")` narrows nothing — `std` is
that root, not a directory inside it. Lua has none to ask for, since `package.path` belongs to
whatever interpreter embeds it and neither a Neovim runtime nor a LuaRocks tree is a standard
library every project shares; Elixir needs none, since `mix` puts the dependencies and their
sources in `deps/` inside the project, where they are project files already, and an installed
standard library is `.beam` files rather than `.ex`. Java, Kotlin,
Ruby and the rest have no roots yet, so `d` stays inside the project for them. `d` on a Go package
qualifier, `db` in `db.Get`, lands on the import line of the open file, `db: via import
code.gitea.io/gitea/models/db`, unless a local or a top-level name of the package is called
that, or the function mentions the name other than as a qualifier. A parameter has no
declaration the rules know, nor has an enum variant unless its class declares it as a field (a
Python `Enum` member, a TypeScript enum member with a value): `d` says so, and `u` lists every
whole-word use of the identifier. On `x.field` the word is a member: in Python, TypeScript and Go
the field of the type `x` is proven to have (below), else every method, property and field of
that name, found by name.

`d` never jumps without saying how it found the target. The status line reads
`delete_user → UserRepository.delete_user (by name, 1 match)` after a jump, `load: via import
json` or `UserRepo: via import app/repos.py` for a declaration an import leads to,
`delete_user → UserRepository.delete_user (via self.repo: UserRepository)` for one the type of the
receiver leads to, and `delete_user: by name, 2 declarations` over a picker, whose title says the
same. `by name` means a declaration pattern matched the word and nothing narrowed which
declaration it is, so a jump by name also says it was the only match. Each picker row starts with
what the declaration sits in and why it is listed —
`UserRepository.delete_user  by name  repos.py:8: async def delete_user(…)` — so typing a class
name filters the rows. A jump leaves the cursor on the name it landed on, so `d` there asks the
next question about it straight away.

What `d` does not claim, in Python, TypeScript and Go:
- On a declaration, the other declarations of the name are namesakes nothing ties to it. They
  are offered under `delete_user: at a declaration, 1 other by name`, even when there is one, so
  pressing `d` again after a proven jump never walks out of the type it has just proven. A field's
  declaration — `poster_id: int` in a class body, the `self.repo = repo` of `__init__`, a struct
  field — offers the other fields and members of its name the same way, on the name it declares
  only: the right-hand `repo` of `self.repo = repo` is the parameter, and the name of a Go embedded
  struct is its type's, where `d` goes.
- A parameter or a local hides an import of the same name: `json` in `def handler(json)` is a
  value, and `d` on the name itself lands on that binding, `helper → f.helper (local)`. In front of
  a dot it keeps the search to members: a function at the top of a module is not one.
- `Outer.find`, with `Outer` a namespace or a class, is what `Outer` declares (`via Outer`), not a
  method `find` of some other class. Rust, C++ and PHP write it `Depot::open`, with modules in
  front of the type only when the path starts inside the project (`crate::`, `self::`, `super::`,
  a file or a directory called so), only for a type the project declares once, and not when a
  `use` of the file binds the path's first name outside it; `Self::open` and a value's
  `shed.open()` stay by name. A
  Python `Limits.MAX_USERS`, `Color.RED` of an `Enum` or a dataclass field is read in the body of
  the class the qualifier names, declared in the file or imported, or of a class above it; an
  attribute a method assigns to `self` is an instance's, a class declared inside the function is
  not read, and a class whose body writes the word in a form the rules do not read (a tuple
  target, a `def` under an `if`) is not passed over for its base.
- An imported name is looked up at the top of the module it comes from, outside the project as
  inside it, and a name imported from two modules (`try` / `except ImportError`) offers both.
- A line inside a triple-quoted string — a Python docstring, an Elixir `@moduledoc` — a Go raw
  string, a template literal, a Lua `[[ ]]` or `[==[ ]==]` long string or block comment, or a
  `/* */` block declares nothing. Each language says which of those forms it has rather than
  inheriting another's: Zig has none at all, since a `\\` string ends with its line, so the
  markdown a `\\` block holds is read as the code it sits in.

On `x.word`, `x.f.word` and longer chains in Python, TypeScript and Go, `d` first looks for the
type of the receiver. `x` is `self` or `cls` in a method, `this` in a class, a Go method's
receiver, a parameter, a local or a module-level variable, and each name after it is a field of
the type before it. The type comes from the declaration:
- an annotation: `repo: UserRepository`, `private repo: UserRepository` (a constructor parameter
  too), a struct field `repo *UserRepository`, `var repo UserRepository`;
- a construction: `UserRepository()`, `new UserRepository()`, `UserRepository{}`,
  `&UserRepository{}`;
- a parameter handed on: `self.repo = repo`;
- a call, one hop through the return type the function declares: `-> UserRepository`,
  `): UserRepository`, `func NewRepo() *UserRepository` (a Go function's first result), or a
  function that declares none and whose every `return` constructs the same class,
  `return new UserRepository()` in TypeScript, `return UserRepository()` in an undecorated Python
  `def` (a bare `return` or a `yield` spoils it). A callee that a parameter or a local of the
  scope names is a value, not the function of that name. The function may be a method called on a receiver
  whose type is proven the same way, `info := e.RequestInfo()` or `repo = self.depot.people()`:
  the method is looked for in that type and the types it extends, where it must be declared once,
  an interface's method line included;
- a cast: `repo = cast(UserRepository, found)` (`typing.cast` too, the type quoted or not;
  a `cast` the project declares itself is a function, read by its return type),
  `const repo = found as UserRepository` (the last type of `as unknown as T`),
  `repo, ok := found.(*UserRepository)`, and the variable of a Go type switch,
  `switch v := found.(type)`, inside a `case *UserRepository:` (a `case` of several types and
  `default` leave it unknown). A chain may hang off the cast itself:
  `(found as UserRepository).deleteUser`, `found.(*UserRepository).DeleteUser`,
  `cast(UserRepository, found).delete_user`, `via found.(*UserRepository)`;
- a loop over a collection whose type is written: `for repo in repos` with
  `repos: list[UserRepository]` (`Sequence`, `Iterable`, `set`, `tuple[T, ...]` and the like),
  `for (const repo of repos)` with `UserRepository[]`, `Array<T>` or `Set<T>`,
  `for _, repo := range repos` with `[]*UserRepository`, `[4]T` or `map[K]T`. The collection is a
  plain name, and every declaration of it writes that type: as an annotation, as the declared
  return type of the function it was assigned from, or in Go as `make([]T, …)`, a literal `[]T{…}`
  or a named type declared `type RepoList []*UserRepository`. `repos.word` on the collection itself is no member of `UserRepository`, and a `dict`'s
  keys, a `Map`'s pairs, `for … in`, a tuple target, a single `range` variable and `async for`
  stay unknown;
- a property with a declared return type: `@property`, `@cached_property` or
  `@functools.cached_property` over `def users(self) -> UserRepository`, a getter
  `get users(): UserRepository`;
- a TypeScript destructuring out of a chain of names, `const { repo, audit: trail } = this` or
  `= this.uow`, on one line or wrapped over several: the field's type. A default, a rest, a nested
  or an array's pattern is a binding of no readable type.

`T | None`, `Optional[T]`, `Annotated[T, …]`, `T | null` and generic arguments read as `T`.
The innermost scope that declares `x` decides: in Python the function the cursor is in (every
binding in it, before the cursor or after), else the nearest enclosing function that binds the
name, else the module; in TypeScript and Go the nearest block around the cursor with a declaration
above it, a function's parameters counting with its body. So a local hides a module-level name, a
closure's variable the one of the function around it, a block's the function's. The declarations
of that one scope must all read the same type, and one that reads none (a loop variable, a
parameter with no annotation) hides the outer ones all the same, so nothing is guessed: two
assignments in the branches of an `if` are a picker. A Python binding need not start its line:
`if fresh: ledger = A()`, `a = 1; ledger = A()` and `first = ledger = A()` (no type read) bind. In TypeScript and Go a header hides the
outer scopes only with what it binds for the block under it: the loop or the `catch` it is, the
function whose body it opens. The parameter of any other function on those lines,
`if (repos.some((repo: Repo) => …)) {` or `register((repo: Repo) => repo, {`, and of one on the
cursor's own line, counts and hides nothing, since the cursor stands outside it; and what the
`if` branch declares is nothing to its `else`. A line inside a docstring, a raw string or a
template declares nothing. A TypeScript class header prettier wrapped is one header: a list of
type parameters that ends in `> extends Base<K> {`, the clauses over a lone `{`; `this`, `super`,
the fields and what the class extends are read through it, and `new Local.Tool()` is the `Tool`
inside `namespace Local` of the file or of the import, and an import of `HonoBase` finds the class a
module declares as `Hono` and hands out with `export { Hono as HonoBase }`. A Go name no scope of the file declares is the package's: a `var`
of any file of the package, alone or in a `var (` block, above the cursor or below it, and two
files that declare it as different types (build tags) agree on nothing. An empty scope walk is no
proof that there is no local, since the walk does not read every form of one: the function around
the cursor must not mention the name anywhere other than in front of a `.` or a `)`, and the file must not
import it. The type must be
declared once, in
the same file, the same Go package or the project module an import names. Go's `type X = Y` is
`Y`, through another alias and another package, unless methods are declared on `X` itself, which
then answers under its own name; `type X Y` is a type of its own. Of a Go
declaration written once per platform, `clock_windows.go` beside a `//go:build !windows` file, the
one the host's `go build` compiles counts: the `_GOOS` / `_GOARCH` ending of the file's name and
its `//go:build` line decide, read as a plain `go build` reads them: the platform, `cgo`, `gc` and
the releases (`go1.21`) are set, a tag of the project's own (`gogit`, `bindata`) is not, so of
`repo.go` (`!gogit`) and `repo_gogit.go` the first counts. `GOFLAGS=-tags=gogit` in the
environment, the variable `go build` and gopls read, turns it around, and `CGO_ENABLED=0` unsets
`cgo`. Only on certainty: every declaration's file must be known to be built or known not to be.
The older `// +build` line is not read, and from inside a file that is not built (`repo_gogit.go`
itself) nothing is preferred: those stay a picker. `d` then looks for the
member in that type, and in the classes it extends and the structs it embeds, and the status line
names the link: `via self.repo: UserRepository`, `via NewRepo() *UserRepository`,
`via makeAudit() returns new AuditLog()`. A member that is no method is a field, and `d` lands on
its declaration — a class-body `poster_id: int`, a constructor parameter
`private repo: UserRepository`, a struct field `PosterID int` or an embedded struct — in the type
or the nearest one it extends or embeds: `PosterID → Issue.PosterID (via issue: Issue)`. A field
that no type declares that way is declared by its first `self.repo = …` in the base-most class
that assigns it, so a later `self.repo = other`, in the class or a subclass, goes there too. A
`this.x = …` counts only where `this` is the class, not an object literal or a `function` inside a
method; a parameter behind a modifier only in a constructor; a line of a docstring never. On an
interface or a base class `d` lands on the declaration there; a second `d`, with the cursor on it,
lists what implements it.

`super().word` in a Python method and `super.word` in a TypeScript class are `self` / `this` with
the walk started one level up, so an override leads to what it overrides:
`store → Archive.store (via super of ColdArchive)`. Under several Python bases only what needs no
method resolution order is proven: the first base declaring the member itself, or every base
leading to the same declaration (`Generic[T]`, `Protocol`, `ABC` and `object` aside), at every
level the member is looked for. Bases that disagree, or one outside the project that may declare
the member first, leave the word to the search by name, and so do `super()` in a function inside
the method and a local assigned from `super.make()`, whose return type an override may narrow. Go has no `super`: its
`i.Base.Touch()` is a chain through the embedded struct.

A chain is followed the same way one field at a time, up to six names in front of the word: on
`self.uow.users.delete_user` the type of `self.uow`, then the field `users` in that type, then
`delete_user` in the type of `users`. A field may be declared in a class the type extends, or
promoted from a Go struct it embeds, and an embedded struct can be a link by its name
(`h.Deps.uow`). The status line lists the links:
`delete_user → UserRepository.delete_user (via self.uow: UnitOfWork → users: UserRepository)`.
A TypeScript chain is read as the language means it: broken by prettier in front of its dots
(`return this.db` over `.selectFrom(`) it is one line, `repo!.find` and `uow?.users.find` are
`repo.find` and `uow.users.find`, and a `#private` name is the word with its `#`, on the `#` and
on the name, never the public name beside it.

A type declared outside the project or any link the rules cannot prove leaves the word to the
search by name below. With two or more names in front of the word the status line says where the
chain broke: `delete_user: by name, 2 declarations (chain broke at item)` when `item` is typed by a
generic parameter, at a property or a getter with no return type, or at the seventh name of a
longer chain. A chain may hang off the call that starts the expression, which is read as a call
assigned to a name would be: `make_uow().users.delete_user` is
`via make_uow() -> UnitOfWork → users: UserRepository`, and so are `pkg.New(x).Run`,
`new Depot().people` and `self.repos.users.get_one(id).name`. A call of a call,
`open_depot().people_repo().delete_user`, is not followed: it is a member of a value whose type is
not known.

When the type of `x` is not known, every method of that name is a candidate: Python `def` and
`async def` inside a class, TypeScript class and object-literal methods, properties holding a
function and bodiless signatures, Go `func (r *T) Name(`. They are collected from the project and
from the standard library and dependencies, where TypeScript is read from its `.d.ts` files only.
So is every field of that name in the project, one row per type, on the line a proven receiver
would land on: Python `name: T` or `name = …` in a class body and `self.name = …` in a method,
TypeScript members and constructor parameters behind a modifier and `this.name = …`, Go struct
fields and embedded structs. A local, the key of a dict or an object literal and a line of a `var`
block are no field, and a name several types declare, such as `id`, is a picker rather than a
jump. The field lines are searched apart from the methods, and when they fill the search the count
says `+` and a single candidate is offered rather than jumped to. Fields outside the project are
not collected: there a field name is every `name: string;` of every `.d.ts`. A project with no
such method or field has the word at its top level instead — `x` was a class or a
namespace — and the usual declarations answer. One candidate jumps; several open the picker, the
project's first. A bare `self.word` or `this.word` whose class, or a class it extends, cannot be
read gets the project's declarations of that name, fields included, and nothing outside the
project.

With the cursor on the declaration of a member — a `Protocol` method, an interface signature, a
method of an abstract or a plain base class — `d` offers what implements it instead, labelled `send:
implementations of Notifier.send, 3 declarations`. In Python and TypeScript that is the types that
name it: the subclasses, the classes and interfaces that `extends` or `implements` it, and the
subclasses of those, four levels down. A type counts only when its own header names one of them and
its file can see that name — it declares it, or an import binds it, followed through a barrel's
`export * from` and `export { Name } from` to the file that declares it — and only when it
declares the member itself, so a class that inherits it is not listed. A Python `Protocol` and a Go interface
name nothing, so there the rule is structural, the way both languages mean it: every member of that
name taking the same number of parameters. Only the project is searched, since an interface is
opened to find what this project does with it. A TypeScript header wrapped over several lines is
read whole, as prettier writes `export class X` over `  extends Base` over `{` (a type
parameter's `S extends Notifier,` is a constraint and implements nothing), and a `#private` member
has no implementations; so is a Python one, whose bases stand under the `class` line. Nothing
implements the declaration — a method beside its Go
type, a class with no subclasses — and `d` goes on to the search by name below.

## What `d` recognises, language by language

| File | What `d` recognises | Searched |
|---|---|---|
| Python | `def` and `async def`, `class`, module-level assignment (annotated or not); behind a dot, a field: `name: T` or `name = …` in a class body, `self.name = …` in a method | every `.py` file |
| Go | `func` with or without a receiver, `type`, `var`/`const`, `:=`; behind a dot, a struct field or an embedded struct | every `.go` file |
| TypeScript / JavaScript | `function`, `class`, `interface`, `type`, `enum`, `namespace`, `const`/`let`/`var` (so arrow functions assigned to a name), class and object-literal methods, properties holding a function, a method signature with a return type and no body (`find(id: string): User;` in an interface, an abstract class, an overload or a `.d.ts`), behind `export`/`default`/`declare`/`async` and the member modifiers; behind a dot, a field: a member `name: T;` or `name = …`, a constructor parameter behind a modifier, `this.name = …`. Destructuring and parameters have no rule. | every `.ts`, `.tsx`, `.js`, `.jsx` and friend: they search each other |
| Rust | `fn`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`, `mod`, `macro_rules!`, `let`, behind any `pub(..)`/`async`/`unsafe`/`const`/`extern`/`default` prefix. `impl` blocks count as uses. | every `.rs` file |
| Java | `class`, `interface`, `enum`, `record`, `@interface`; a method, an abstract or interface method and a field, told from a call by the return type before the name — a primitive, or a name with a capital in it, as Java writes its types; a constructor, behind at least one modifier, since a bare `Name(x) {` is a call. Annotations and modifiers may stand in front of any of them. | every `.java`, `.kt` and `.kts` file: they search each other |
| Kotlin | `fun` (with the receiver of an extension function), `class`, `interface`, `object`, `enum class`, `typealias`, `val`/`var`, behind `private`/`open`/`data`/`sealed`/`suspend`/`override` and the rest | every `.java`, `.kt` and `.kts` file: they search each other |
| Ruby | `def`, `def self.name`, `class`, `module`, an assignment (a constant, an `@ivar`, a local), `attr_accessor`/`attr_reader`/`attr_writer`, `alias`/`alias_method`. A trailing `?` or `!` is not part of the word, so `d` on `empty?` finds `def empty?`. Rails-style DSL (`scope`, `has_many`) has no rule. | every `.rb`, `.rake`, `.gemspec`, `.podspec`, `.rbi`, `.ru` file and `Rakefile`, `Gemfile`, `Vagrantfile` and friends |
| C / C++ | a function, a prototype and an out-of-line method (`Type::name(`) in column zero, where the languages have no statements, so a call is never one — the return type may sit on the line above, as GNU style writes it; a method or a function indented, when its body opens on the line; `struct`, `class`, `union`, `enum`, `enum class`, `namespace`, behind a template head, a storage specifier and an attribute or export macro (`struct __attribute__ ((__packed__)) sdshdr8`, `class FMT_API name`), a template specialization included; `typedef` in every form, `using x =`, `#define` (function-like too), a global. A header's prototype is offered next to the definition, in the picker's usual order, by path. An enum constant has no rule — `NAME,` in an `enum` body and in an initializer list are the same line — nor has a field, a local, a template parameter or a member function only declared inside its class. | every `.c`, `.h`, `.cc`, `.cpp`, `.cxx`, `.hpp`, `.hh` and `.hxx` file: they search each other |
| C# | `class`, `struct`, `interface`, `enum`, `record`, `record class`, `record struct`, `delegate`, past the generic parameters they declare and behind `[Attribute]` lists and any modifiers (`public sealed partial class Foo<T>`); a `namespace`, under its last part; a `using x =` alias; a constructor, behind at least one access modifier, since a bare `Invoice(n)` is a call; and a method, a property, an event, a field or a local, told from a call by the type before the name — a predefined one, `var`, or a name with a capital in it, as C# names its types — so `public int X { get; }`, `public string Name => _name;` and `int IComparable.CompareTo(o)` all count. An enum member has no rule: `Open,` in an `enum` body and in a collection initialiser are the same line. | every `.cs` and `.csx` file |
| Swift | `class`, `struct`, `enum`, `protocol`, `actor`, `typealias`, `associatedtype`, `extension Type` — where a project keeps its own members of a type, often the only place — `func` past its generic parameters, `init`, `init?`, `subscript` and `deinit`, `let` / `var`, and an `enum` case, alone or among several on a line, with the associated or raw value it carries. All of them behind their `@attributes` and any modifiers (`public final override class func`, `private(set)` included), and a backticked name counts. A `case .open:` or `case let .open(x):` of a `switch` is a pattern, not a declaration, and a binding made by `if let` / `guard let` has no rule: it rebinds a name declared elsewhere. | every `.swift` file |
| PHP | `function` (`&` included), `class`, `interface`, `trait`, `enum`, a `const` and a `define('X', …)`, an `enum` case, a property with the type it carries, and a constructor parameter promoted to one — all behind their `#[Attribute]`s and modifiers (`final public static function`) — plus an assignment that opens a line (`$x =`, `.=`, `??=`, `+=`). A `case X:` of a `switch`, a `$key => $value` pair, `$rows['x'] =` and `$this->name = …`, which writes to a property declared elsewhere, are not declarations, and a `foreach` target and a parameter have no rule. | every `.php` and `.phtml` file |
| Lua | `function name(`, `local function name(`, `function M.name(`, `function M:name(` and the longer `function a.b.name(`; a function literal bound to a name (`M.name = function(`, `name = function(` in a table of handlers); `local name`, one of several on the line included. A field holding anything else has no rule: `limit = 10` in a table constructor and a re-assignment inside a body are the same line, and the language has no keyword to tell them apart. | every `.lua` file |
| Elixir | every `def` form — `def`, `defp`, `defmacro`, `defmacrop`, `defguard`, `defguardp`, `defdelegate` — written `def name(x) do`, `def name do` or `def name, do: x`, a trailing `?` or `!` included; `defmodule` and `defprotocol` under the namespace they are written with, by their last part, so `defmodule MyApp.Repo` declares `MyApp.Repo` and nothing called `MyApp`; a `defstruct` field, atom list or keyword form, on the `defstruct` line itself — a field on a continuation line of a struct written over several lines has no rule, since that line is the shape of any keyword list; a module attribute where it is given a value (`@timeout 5_000`). Several clauses of one function are several declarations and all are offered. `@spec`, `@type` and the rest of the attributes the language and the libraries everyone uses own — ExUnit's `@tag`, Mix's `@shortdoc` — are directives: `@spec parse(t) :: t` is no declaration of `parse`, and `d` on one of those names has nothing to find. That is a list of known names, which is all a line pattern can have: any library may define an attribute, and `@tag :slow` and `@timeout 5_000` are the same line. `defimpl` declares the module `Protocol.Type`, where neither half is a name of its own, as a Rust `impl` is not. | every `.ex` and `.exs` file |
| Zig | `fn name(`, behind `pub`, `export`, `extern "c"`, `inline`, `noinline`; `const` and `var`, which is how the language declares a type (`const Ledger = struct {`, `const Status = enum {`, `const Value = union(enum) {`), an import, a constant and a local alike, `threadlocal` and `comptime` included. A struct field (`total: u32,`) has no rule, as a C field has none: it is the shape of a value in a struct literal. Neither has a `test`: a word inside its description declares nothing, so `d` can never land there — `D` lists the tests instead. | every `.zig` file |
| Shell | `name()` and `function name`, an assignment behind `export`/`declare`/`local`/`readonly`/`typeset` (or bare, and `+=`), `alias` | every `.sh`, `.bash`, `.zsh`, `.ksh` and shell dotfile (`.bashrc`, `.zshrc`, `.profile` and friends) |
| SQL | `CREATE` of a table, view, index, function, procedure, trigger, type, schema, sequence, domain, extension, database, role or user, behind `OR REPLACE`, `TEMP`, `UNLOGGED`, `MATERIALIZED`, `UNIQUE` and `IF NOT EXISTS`, schema-qualified or quoted; a `WITH … AS (` common table expression. Keywords ignore case. Columns have no rule. | every `.sql`, `.psql`, `.pgsql`, `.mysql`, `.ddl` and `.dml` file |
| Makefile, `*.mk` | a target, also one of several before the colon; a variable | every Makefile |
| Terraform | the block behind `var.x`, `module.x`, `local.x`, `data.T.N`, `T.N`; a bare name, as in `.tfvars`, is any block with that label | `.tf` files in the same directory |
| Dockerfile | the `FROM … AS name` stage | the same file |
| YAML | the `&name` anchor, a key that opens a block (compose services, CI jobs) | the same file |

In Makefiles, Terraform, Dockerfiles and YAML a `-` is part of the word under the cursor, and `d`
in Terraform reads the whole dotted address, so it works from anywhere in `aws_s3_bucket.logs.id`.

## `D`: project symbols

`D` lists every declaration a single regex can recognise (`def class func function type fn struct
enum impl trait interface mod const static union macro_rules! namespace`, with
`export`/`pub`/`async`/`const`/`extern`/`declare` prefixes; `const` and `static` only unindented or
exported, since indented they are locals), plus shell functions (`name()`; the `function name` form
the single regex already finds), SQL `CREATE`d objects under the name as written (`public.orders`,
not CTEs), Makefile targets, Terraform blocks by address (`aws_s3_bucket.logs`, `data.T.N`,
`module.x`, `var.x`, `output.x`), Dockerfile stages and YAML anchors, each read only from its own
kind of file; recomputed on each press. Zig adds a function behind `inline` or `noinline` and a
`test`, under the description it is written with, which that regex has no word for. Java, Kotlin,
Ruby, C, C++, C#, Swift, PHP, Lua and Elixir are read from rules of their
own instead of that regex — Java's types and its methods, told from a call by the return type before
the name; Kotlin's `fun` (past an extension's receiver), types, `object`, `typealias` and
`const val`; Ruby's methods, classes and modules, `def self.name` included; C and C++ functions,
methods, types, `typedef`s, `using` aliases and `#define`s; C#'s types, delegates and namespaces
behind their attributes and modifiers, and its methods and properties, told apart from a call the
way Java's are; Swift's types, `protocol`s, `actor`s, `typealias`es, `func`s and `extension`s, an
extension under the type it extends; PHP's types and `const`s in one row and its functions and
methods in another, behind `final public static` and the rest; Lua's functions in both of the
forms it writes them, under the name and not the table they hang off; Elixir's modules, protocols
and every `def` form — so none of them is listed twice or
under a modifier or a receiver. A C prototype is not listed, since every function of a header would
be there twice, and a `typedef struct x { … } y;` is listed once, under the `y` the project writes.
TypeScript's class methods, with neither a keyword nor a type in front, are not listed: the regex
cannot tell `name(` from a call. Neither are fields, a C or Zig global, a Lua local, a C#
constructor (its class is already a row), a Swift `let`, `var`, `init` or `enum` case, a PHP
property, `enum` case, `define()` or magic method (`__construct`, `__toString`: the language's
hook, not the project's), a Ruby
constant, an Elixir module attribute or `defimpl`, or the names a Ruby `attr_accessor` or an
Elixir `defstruct` line declares, since one line can declare several.
Past 5,000 declarations the grep stops, so the list is only what it reached in file order: the
title counts those rows and says what they are (`Symbols (first 5232, type to search all)`), and
the query stops filtering them and greps the project for a declaration whose name it matches,
after a pause in the typing, the way `s` does. A name declared in a file the cut never reached is
found that way; the title then counts the answer (`Symbols (94 hits)`, `Symbols (5000+ hits)` for
one the cut caught too, `Symbols (…)` while the grep runs). What the query matches there is the
declared name as typed — not the path beside it, and not the picker's own pattern syntax, so
`^`, `!`, a space and an accent are characters to find. Under 5,000 the rows are the whole list
and the query filters them, as before.

## `/` and `s`: search

Searches ignore case, capitals in the query included: `sameCancel` finds `SameCancel` in `/`,
`s`, `D` and `o` alike. `/` and `s` look for the text as typed: `foo(` finds the calls and the
definition, `a.b` only `a.b`. There is no regex mode and no case switch; `u` lists the uses of a
word in its exact case. `s` lists its hits while you type, the open file's first; Up / Down pick
one and Enter jumps to it.

## `u`: usages

`u` lists every whole-word use of the identifier, case-sensitive, in the order a reader wants
them: the declarations of the word, told by the same rules `d` uses — its patterns, and not a line
inside a docstring, a raw string or a block comment — and marked `declaration` in the row, then
the open file, then the rest of the project's code with the nearest directories
first, and last the tests, mocks, fixtures, generated and vendored files — a `test/`, `tests/`,
`__tests__/`, `spec/`, `specs/`, `testdata/`, `fixtures/`, `mocks/` (also `__fixtures__/`,
`__mocks__/`), `vendor/` or `third_party/` directory anywhere in the path, and the file names
`test_*`, `conftest.py`, `*_test.*`, `*_spec.*`, `*.test.*`, `*.spec.*`, `*_pb2.py`,
`*_pb2_grpc.py`, `*.pb.go`, `*.gen.go` and `*.generated.*`. The title says how the list splits —
`Usages of delete_user: 1 declaration, 6 in code, 14 in tests` — and leaves out a part with no
hits. The file on screen is never demoted, whatever it is called, and the candidates `d` offers
are demoted by the same table, so a copy of a declaration under `spec/` is offered after the real
one.

## The project is live

The file list comes from a `.gitignore`-respecting walk, and the project on screen is the project
on disk: merl watches the root, and a file that an agent in the next pane creates, deletes or
renames is in the tree, in `o` and in what `s`, `u` and `d` search a moment later, with no key and
no restart. The tree cursor stays on its entry, expanded directories stay expanded, an open picker
keeps its rows until it is reopened, and an edited `.gitignore` is picked up. Dotfiles are part of
the list — `.github/`, `.env`, `.dockerignore` — and only the `.git`, `.hg` and `.svn` stores are
skipped.

What `.gitignore` leaves out is still in the tree, dim, as in VS Code's Explorer: `.env`,
`target/`, `node_modules/`. The walk does not go into an ignored directory; it is one row until it
is expanded, and then it is read from disk one level at a time. While it is expanded it follows the
disk like the rest of the tree: what `npm install` or an agent writes there shows up. Collapsed, it
is silent again, and read anew when it next opens. `o` offers the ignored files that
sit in a directory the walk went into, `.env` or `config/local.yml`, dim and after the rest, but
nothing from inside `node_modules/`. `s`, `u` and `d` search only what is not ignored.

The open file itself is watched and reloads on every change on disk, keeping the cursor, the
scroll position, the jump history and the undo history, where the change is one more step.
