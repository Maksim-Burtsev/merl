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
`baseUrl` of the nearest `tsconfig.json` (or `jsconfig.json`) and the configs it extends. A named import finds that
name, aliased or not; a default import finds the declaration under its local name, or else the
module's `export default`; `ns.x` behind `import * as ns` finds `x`. A Go import path below the
`module` of a `go.mod` in the project is that package's directory. The word must be declared
directly in the module, or in the class the chain goes through (`UserRepo.create`), so `store.Open`
never lands on a method called `Open`. The status line names the file or the package:
`UserRepo: via import app/repos.py`, `Open: via import store/`. A Java or Kotlin import names a
package by its dotted path, and the project's packages are what the `package` lines of its
`.java`, `.kt` and `.kts` files declare (Kotlin ties no package to a directory): `import
app.a.User` is looked for in the file `User.java` of the package `app.a`, or in any Kotlin file
of it; `import static a.b.C.m`, `import static a.b.C.*` and `import a.b.Outer.Inner` look inside
`C` or `Outer`; a qualifier bound so narrows the same way, `User.find`; and a Kotlin `import
a.b.C as D` finds `C`. A capitalised name no import binds is the file's own package's first, then
that of each project package a wildcard `import a.b.*` names, all of them. On an import line a
package segment (`halo` in `import run.halo.app.User`) declares nothing. A Python module of the project that
does not declare the word but imports it — a package's `__init__.py` — hands it on: its own
module-level imports are followed, under another name and through `from .labels import *` too,
four modules deep, and the status line names the file the word ends up in. Two sources, a name
the module also assigns, an import inside a function and a cycle are not followed. A TypeScript
module that does not declare the word hands it on too: a barrel's `export { Name } from "./x"`,
`export { default as Name }`, `export { x as Name }` and `export * from "./x"` are followed to the
module that declares it, several `export *` sources that do are offered in a list, and a module
whose `export default Name;` exports what it imports is followed to that import, four modules deep
(`export default observer(Name)` stays where it is). Any other module that does not declare the
word itself is not followed further: `d` falls back to the search by name below and says `by
name`. Behind
`from repos import UserRepository as Users` a receiver typed `Users` is a `UserRepository`.

A Python builtin has no source on the machine: the interpreter has it compiled. `d` on a bare
name of `dir(builtins)` — `next`, `map`, `ValueError` — that nothing in the file binds (no
parameter or local of the scope, no module-level `def`, `class` or assignment, no import, and no
`from x import *`) says `next: builtin, no source` and stays where it is, with no picker and no
search; a project function of that name in another module is not what the bare name means. So
does a member of a value proven to be a builtin type (`str`, `bytes`, `int`, `float`, `bool`,
`list`, `dict`, `set`, `tuple` and the like, `list[int]` included, but not `typing.List`) that the
file neither declares nor imports: `replace: builtin, no source (via render() -> str)`. A bare
name nothing binds that is no builtin is never a method outside the project: it is looked for
there only at the top of the modules the file `*`-imports.

A Python module's name lands on the module, at its first line: `repos: module app/repos.py`. That
is a word in the module path of an import line (`app` or `repos` in `from app.repos import
UserRepo`, `json` in `import json`), and a name an import binds to a module (`views` behind
`from shop import views`, `json` in `json.dumps`), in the project and outside it. A package comes
before a module of the same name beside it, as Python imports it. Outside, the module is matched
from the root it lies under, `json/__init__.py`, `json.py` or `json.pyi` there, never a `json.py`
deep in another package; a package that binds the name itself (`serializers = …` in its
`__init__.py`) keeps its say. A word of an import's path that names no module, and the name of a
plain `import x` where `x` is not installed, get `no definition`: they can only be modules, so no
namesake is searched for.

In Rust a bare name, with no `.` or `::` in front, is the item the file declares under it where
the cursor sees it: a `fn` nested in the function, an item of the inline `mod` around the cursor
or of the file's top level, and inside a `mod tests { use super::*; … }` the file's own after the
block's. `d` jumps there and says `in this file`, since another file's items are out of sight
without a `use` or a path. The head of a path, `name::…`, is only a type or a module. Wherever
something else may be what the name means, it is looked up as before: a name a `use` in sight
imports, a generic parameter, an item of a block or of an outer function, and a lowercase name the
function mentions other than as a call, `name(`, `name!` or `name::`, which may be a local. `Type::new` where the file declares
`Type` and another crate one too looks for `Type::new` in this file first, `via Type`.

A Rust path's first name, the one a `use` of the file binds or the one written out, names the
crate looked in first: `crate`, `self` and `super` the project's crate and module, a `[package]`
or `[lib]` name of a `Cargo.toml` of the project that crate, `std`, `core`, `alloc` and
`proc_macro` the sysroot's, any other name a crate of `Cargo.lock`. In the crate, `k::a::b::w` is
looked for in `src/a/b.rs` or `src/a/b/mod.rs`, then in the files under `src/a/b/`, then in the
whole crate, and `File::open` behind `use std::fs::File` keeps only `File`'s `open`, `via import
std::fs`; what `std` does not declare is looked for in `core` and `alloc`, and `usize::MAX` in
`core`, by name. Only a path whose crate declares nothing of it goes on to the search by name.

An import of anything else is looked for outside the project first, even when the project declares
a word of the same name. A word no import binds goes
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
stub. A Python module is matched from the root it lies under (#329): `json` is `json.py`,
`json.pyi` or `json/` right under a root, never `kombu/utils/json.py`. A module outside that does
not declare the name hands it on through its own module-level imports, followed as the project's
are, four modules deep: `pytest.mark` is `mark: via import _pytest.mark.structures`, through
`pytest/__init__.py`'s `from _pytest.mark import MARK_GEN as mark` and the package's relative
import. A source with no file, a compiled `_io`, ends on the import line that binds the name
(`io.py`). Only when the imports lead nowhere is the name looked for everywhere outside, by name,
and then offered, never jumped to. Under a `.venv` the base interpreter's `site-packages` is left
out unless `pyvenv.cfg` sets `include-system-site-packages = true`, and a root inside another
(`sys.path` lists `lib/python3.11` and its `site-packages`) is walked once. The standard library's hits come before the dependencies', the picker shows paths relative
to their root, and files opened from there are read-only. Go's `_test.go` files, `testdata` and
nested modules such as GOROOT's `cmd` are skipped, since no import reaches them. C and C++ have no
per-project manifest — what the build system was told with `-I` is not in the source — so their
roots are the system headers: the SDK `xcrun` reports on a Mac, `/usr/include` on Linux, and
`/usr/local/include` and `/opt/homebrew/include`. Objective-C — a `.m` or `.mm` file, or a header
with an `@interface`, `@protocol`, `@class` or `#import` line — reads besides them the `Headers` of
every framework of that SDK, `System/Library/Frameworks` and UIKit's
`System/iOSSupport/System/Library/Frameworks`, each header once, `<Foundation/NSString.h>` being
`Foundation.framework/Headers/NSString.h` for the headers an `#import` reaches; and CocoaPods'
`Pods/`, which a project gitignores as it does `node_modules`. A C or C++ file reads none of them
(#417). `#include` and `#import` bind no name of their own, so nothing
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
that root, not a directory inside it. PowerShell has the module directories of `PSModulePath`,
or without it PowerShell 7's defaults on macOS and Linux, `~/.local/share/powershell/Modules`,
`/usr/local/share/powershell/Modules` and the `Modules` beside the `pwsh` on the PATH; the built-in
cmdlets are compiled and have no source there, and `Import-Module` binds no name of its own, so
nothing narrows the search. Dart has the packages `dart pub get` or `flutter pub get` lists in
`.dart_tool/package_config.json` (of the project, or of a package one or two directories down, as
a Flutter app's `app/` is), each under its `rootUri` — the pub cache's
`~/.pub-cache/hosted/pub.dev/<name>-<version>/lib/`, the Flutter SDK's for `flutter` and
`sky_engine` — read as JSON with nothing run, and the `lib/` of the SDK of the `dart` on the PATH,
links followed, or in a Flutter install the `bin/cache/dart-sdk` beside it; no `pub get` yet, no
packages. CMake has its own modules, as source: the `Modules` directory beside the `cmake` on the
PATH, links followed (`<prefix>/share/cmake/Modules` from Homebrew, `<prefix>/share/cmake-X.Y/Modules`
on a Linux distribution), and the packages' config files under `lib/cmake` of `/opt/homebrew`,
`/usr/local` and `/usr`, and `/usr/lib/<triplet>/cmake` on Linux, links followed. An `include`
binds no name, so nothing narrows the search: `d` on `FetchContent_Declare` lands on its
`function(` in `Modules/FetchContent.cmake`, read-only. A plain `import` binds no name, as Swift's does, so nothing narrows the search; a prefix
of `import '…' as p` and a name of `import '…' show A` are looked for in the file the import names
first — `package:<name>/x.dart` is the project's own `lib/x.dart` when `<name>` is the `name:` of
the nearest `pubspec.yaml` above the file, else that package's `lib/x.dart`, `dart:async` the SDK's
`lib/async/async.dart` — `via import <uri>`; a barrel's `export` is not followed, and a file that
does not declare the name leaves it to the search by name. Protocol Buffers has the `include` directories `protoc`
installs its well-known types into — `/opt/homebrew/include`, `/usr/local/include` and
`/usr/include` — walked through the links Homebrew puts there; buf's module cache keeps
dependencies under hashed directories no import spells, and is left out. `d` on the path of an
`import` (`public` and `weak` too) opens the file it names: the path is relative to a proto root,
never to the importing file, so it is the project's file whose path ends with it — several are a
picker — then the first root that has it. A qualified type is resolved as `protoc` resolves it:
its first part in the file's own `package`, then in each package around it out to the root (a
leading `.` starts there), so `v1.User` inside `package shop.v1` is `shop.v1.User`. The longest run
of parts that is some files' `package` narrows the search to those files, `Timestamp: via
google.protobuf`, and the parts after it are the messages the name is nested in; a qualifier that
is no package is a nesting (`Outer.Inner`), looked for as `Outer.find` is. Lua has none to ask for, since `package.path` belongs to
whatever interpreter embeds it and neither a Neovim runtime nor a LuaRocks tree is a standard
library every project shares. Elixir has the `deps/` that `mix deps.get` fetches the
dependencies into, as source, beside the `mix.exs` of the file's project or of the umbrella above
it: `mix new` gitignores it, so it is outside the project walk the way `node_modules` is, and the
picker shows it from the project root, `deps/jason/lib/jason.ex`. A module is no path
(`Phoenix.LiveView` lives in `phoenix_live_view/lib/phoenix_live_view.ex`), so a qualifier such
as `Jason` in `Jason.encode!` narrows the search to the file of `deps/` that declares
`defmodule Jason`, else to the rest of its package, and that comes before a namesake the project
declares; a qualifier no dependency declares, `Enum` or `String`, finds nothing outside, since an
installed standard library is `.beam` files rather than `.ex`. Java, Kotlin and Scala have no
roots: the JDK's `src.zip` and Gradle's, Coursier's and Ivy's `-sources.jar` files are archives, so a name imported from a
package no project file declares (`java.util.Objects`, `androidx.compose.ui.res.stringResource`)
answers `no definition`, never a namesake of the project. Ruby has the gems `Gemfile.lock` names
(#369), read and never run, since `bundle` would evaluate the `Gemfile`: the `specs:` of its `GEM`
and `GIT` sections, at the versions it locks, in the first gem directory that has them — the
project's `BUNDLE_PATH` from `.bundle/config`, then `GEM_HOME` and `GEM_PATH`, then those of the
Ruby that `.ruby-version` names under rbenv, mise, asdf or chruby, else of the `ruby` on the PATH,
asked from `/`. A gem is read from its `lib`, a `GIT` gem from Bundler's checkout. That Ruby's
standard library comes first, and before it the core, which is written in C: the signatures in
`core/*.rbs` of the newest `rbs` gem, `def fetch: …` in `class Hash`. With no lockfile, or no gem of
it installed, there are no roots. `require` binds no name and a constant names no file, so a word no
project declaration answers is looked for by name in all of them, a local of a gem's method never
among them: a bare call, `Const.meth` the
class does not declare (ActiveRecord's `find`), `Errno::ENOENT`, and a member on a value, whose
candidates outside join the project's. Nix has none on purpose: nixpkgs and a flake's inputs live
in the store under a hash (`/nix/store/<hash>-source`) that no line of the project spells and that
only `nix` itself knows, so `pkgs.hello` and `lib.mkOption` stay a search in the project. The rest
have no roots yet, so `d` stays inside the project for them. `d` on a Go package
qualifier, `db` in `db.Get`, lands on the import line of the open file, `db: via import
code.gitea.io/gitea/models/db`, unless a local or a top-level name of the package is called
that, or the function mentions the name other than as a qualifier. A parameter has no
declaration the rules know, unless the scope walk of Python, TypeScript, Go or C# binds it (below), nor has an enum variant unless its class declares it as a field (a
Python `Enum` member, a TypeScript enum member with a value) or it is written behind its enum (a
Java or Kotlin `Offer.CUT`, when the project declares one type `Offer` and it is an `enum`) or it
is Rust's: `Mode::Auto` is the variant `Auto` of the `Mode` the project declares once, and a bare
`Auto` is when a `use …::Mode::*;` of the function, else of the module, brings it in. Elsewhere
`d` says so, and `u` lists every whole-word use of the identifier. On `x.field` the word is a member: in Python, TypeScript, Go, Swift and PHP
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

What `d` does not claim, in Python, TypeScript, Go, Java and Kotlin:
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
  string, a template literal, a Lua `[[ ]]` or `[==[ ]==]` long string or block comment, a Rust
  string, raw (`r#"…"#`) or not, a Ruby heredoc (`<<~SQL`) or `=begin` block and what follows
  `__END__`, a C++ raw string (`R"( … )"`, `u8R"sql( … )sql"`), a Kotlin raw string, a Scala
  multi-line string or a Java text block (`"""`), or a `/* */` block declares
  nothing. A word inside a Rust string names nothing either: `d` there says `no definition` at
  once, save on the `{name}` a format string captures. Each language says which of those forms it
  has rather than inheriting another's: Zig has none at all, since a `\\` string ends with its
  line, so the markdown a `\\` block holds is read as the code it sits in.

In C# (#352) `d` reads the type C# writes: on `x.word`, `x.f.word` and chains of up to six
names, `x` is `this`, `base`, a local, a parameter (a primary constructor's too) or a field or
property of the type around the cursor, and its type comes from `var x = new T(…)`, `T x = …`,
`T x;`, a field `T _x;`, a property `T X { get; }` or `T X => …`, `foreach (T x in …)`, `catch (T x)`,
`out T x`, a pattern `is T x`, a cast `(T)y` or `y as T`, or a call, one hop through the return
type of a method the project declares once (`Task<T>` and `ValueTask<T>` under `await` read as
`T`). The member is looked for in `T`, then in the bases and interfaces its header names. On
`Name` of an object initializer, `new T { Name = … }`, `new T(…) { … }` or a target-typed
`new() { … }` (`T x = new()`, `return new()` in a method returning `T`, `T X { get; } = new()`),
it is looked for in `T` the same way. A type the project does not declare is the framework's,
and so is its member: `d` says `no definition … in the project`, unless the project declares an
extension method of it, `static R M(this T x)`, which it opens; so does a member the project's
type lacks when a base it may inherit from is not the project's. `dynamic`, a type parameter, a
type declared more than once, a `partial` type (a source generator may write its other part), an
anonymous type, a collection's or a dictionary's initializer and a `with` prove nothing, and
the search by name answers, as before.

On `x.word`, `x.f.word` and longer chains in Python, TypeScript, Go, Rust and Swift (and PHP, below), `d` first looks
for the type of the receiver (Swift: #384, rules at the end of this list). `x` is `self` or `cls` in a method, `this` in a class, a Go method's
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
  `def` (a bare `return` or a `yield` spoils it). In TypeScript every `return` may also name one
  local that each of its declarations in the function constructs as the same class,
  `const repo = new UserRepository(); return repo;`, and that is not assigned again (#354). A callee that a parameter or a local of the
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
  plain name (in TypeScript also a chain, `for (const m of document.memberships)`, typed by the
  last field's declaration, #354), and every declaration of it writes that type: as an annotation, as the declared
  return type of the function it was assigned from, or in Go as `make([]T, …)`, a literal `[]T{…}`
  or a named type declared `type RepoList []*UserRepository`. `repos.word` on the collection itself is no member of `UserRepository`, and a `dict`'s
  keys, a `Map`'s pairs, `for … in`, a tuple target, a single `range` variable and `async for`
  stay unknown;
- a property with a declared return type: `@property`, `@cached_property` or
  `@functools.cached_property` over `def users(self) -> UserRepository`, a getter
  `get users(): UserRepository`;
- a TypeScript destructuring out of a chain of names, `const { repo, audit: trail } = this` or
  `= this.uow`, on one line or wrapped over several, out of a call read as above,
  `const { auth } = useStores()`, or of a parameter by its written type,
  `function Row({ apiKey, document: doc }: Props)` (#354): the field's type. A default, a rest
  (`...rest`, which binds `rest`), a nested or an array's pattern is a binding of no readable
  type.
- a TypeScript annotation that intersects inline type literals, `props: Omit<Props, "x"> & {
  href: string }` (#354): a field one `{ … }` operand declares is the answer, on its line; two
  operands declaring it, or a top-level `|`, prove nothing.
- JSDoc in a `.js`, `.jsx`, `.mjs` or `.cjs` file (#347), read as the annotation it stands for:
  `/** @type {UserRepository} */` right above a `const`, `let` or `var` or on its line,
  `@param {UserRepository} repo` and `@returns {UserRepository}` in the block right above a
  function or a method. A `@typedef {Object} Options` declares a type whose fields are its
  `@property {T} name` lines, and `@typedef {import("./options").Options} Options` is the type of
  that module, a `@typedef` there, a class or a `.d.ts` interface; a class bound by `const Repo =
  require("./repo")` is a type too. The type in the braces is one name, `?T` and `T|null` read
  as `T`, `T[]` and `Array<T>` hand out `T` to a loop; a union of two types, another generic, an
  inline object or function type, `*`, `any`, a block with a blank line under it and a
  `@typedef` its file declares twice type nothing. A `.ts` file's JSDoc is not read.
- Swift (#384): a parameter's annotation (`lhs: Instant`, `with convertible: URLConvertible`), a
  local's or a property's (`let encoder: FormEncoder`), a construction `FormEncoder()`,
  `FormEncoder.init(…)` or with a trailing closure of a type the project declares (a callee it
  does not declare as a type may be a function, and proves nothing), the `-> Type` of the one
  function or method called, a cast `as! T`, `self` and a bare property inside a type's body (the
  type around the cursor), an `if let` / `guard let` of any of these, and a `for x in xs` over
  `[T]`. `T?` and `T!` read as `T`, `a?.b` as `a.b`. The member is the type's own or its
  extensions', then its superclass's; a value reaches an instance member over a `static` one. A
  type the project only extends (`URLRequest`, `Data`) ends the lookup: its extensions that
  declare the member, else `no definition`. A protocol, `any P`, `some P`, a generic parameter in
  scope, a tuple, a closure type, a `typealias` and a type declared twice prove nothing, nor does
  a type that declares none of the member: the search by name decides.

Java and Kotlin (#388, #391) read the receiver `x` of `x.word`, `x::word` and `x.f.word` (six
names at most, `this.f` included) the same way: `x` is a parameter, a lambda's typed parameter, a
loop variable or a local that the scope walk finds, else a field of the classes around the
cursor; its type is written `Line line`, `var line = new Line(…)`, `line: Line` (a constructor's
property too, `?` and generic arguments dropped) or `val line = Line(…)`, and every declaration in
scope writes the same one. The type is the class the file imports from the project, else the one
class of the name the project declares (this file's, then its package's). The member is a method,
a field or a record component of that class or a class it extends or implements in the project,
a field Lombok writes the accessor for (#381), or the project's Kotlin extension `fun Type.word`:
`total → Line.total (via line: Line)`. A type the project does not declare (the JDK, Spring,
AndroidX, a library) has no source to land on: `no definition`, and so does a string literal's
`"a".equals`, unless the project declares an extension of the name on it; one it declares on some
other type leaves the word to the search by name. An array, a type parameter, a type written with
a dot, a delegated property (`by lazy`), a name that a smart cast, a pattern or a cast may narrow
(`x is T`, `x instanceof T`, `(T) x`), a type declared twice, and a class of the project without
the member all stay by name, as before.

In Rust (#377) `self` is the type of the `impl<…> T` or `impl<…> Tr for T` the method sits in (a
trait's default method proves nothing), and a binding the scope walk below finds says what it
holds: a parameter `x: T`, `x: &T`, `x: &mut T`, `x: &'a T`, `mut x: T`, a closure's `|x: T|`,
`let x: T = …`, and a `let` whose whole value is a struct literal `T { … }`, a call of an
associated function of `T` whose `fn` line returns `Self` or `T` (`T::new(…)`; `T::default()`
counts when the project declares no `default` of `T`), or a call `f(…)` of a function declared
once at the top of the file, through a `use` or where the line sees it, read by its `-> T`. `&`,
`&mut`, lifetimes, `Box<T>`, `Rc<T>` and `Arc<T>` are stripped, and `Self` is the `impl`'s type.
The type is the `struct`, `enum` or `union` declared once in the file, else the one a `use`
names, else the project's only one of that name. Each name after `x` is a field of the struct
before it. The word behind a `(` is a method in the `impl T` and `impl Tr for T` blocks of `T`'s
crate, else one with a body in a trait those `impl`s name; without one it is a field of `T`. A
generic parameter, `Option<T>`, `Result<T, E>`, `Vec<T>` and the standard library's other types,
`dyn` and `impl` traits, a type outside the project or declared twice with nothing to tell which,
and a value with `?` or a method call behind its call prove nothing: the search by name stays,
with `(chain broke at x)` past the first name. `via tmpdir() -> TempDir`, `via dir: TempDir`,
`via self.config: Config`.

`T | None`, `Optional[T]`, `Annotated[T, …]`, `T | null` and generic arguments read as `T`.
The innermost scope that declares `x` decides: in Python the function the cursor is in (every
binding in it, before the cursor or after), else the nearest enclosing function that binds the
name, else the module; in TypeScript, Go and C# the nearest block around the cursor with a declaration
above it, a function's parameters counting with its body. So a local hides a module-level name, a
closure's variable the one of the function around it, a block's the function's. The declarations
of that one scope must all read the same type, and one that reads none (a loop variable, a
parameter with no annotation) hides the outer ones all the same, so nothing is guessed: two
assignments in the branches of an `if` are a picker. A Python binding need not start its line:
`if fresh: ledger = A()`, `a = 1; ledger = A()` and `first = ledger = A()` (no type read) bind. In TypeScript, Go and C# a header hides the
outer scopes only with what it binds for the block under it: the loop or the `catch` it is, the
function whose body it opens. The parameter of any other function on those lines,
`if (repos.some((repo: Repo) => …)) {` or `register((repo: Repo) => repo, {`, and of one on the
cursor's own line, counts and hides nothing, since the cursor stands outside it; and what the
`if` branch declares is nothing to its `else`. C# (#345) binds the parameters of a method, a
constructor, a local function, an indexer and an operator, a primary constructor's across its
type's body, a lambda's (`x =>`, `(x, y) =>`, `(T x, U y) =>`), the variables of `foreach`, `for`,
`catch`, `using` and `fixed`, `out var x`, `is T x`, `case T x:`, and `var x =`, `T x =` and `T x;`
statements, a `{` on a line of its own belonging to the header above it. A deconstruction binds
nothing, a type's body declares no local, and a lambda's parameter on the cursor's own line binds
inside that lambda only. A line inside a docstring, a raw string or a
template declares nothing. In Rust the nearest binding above the cursor in the blocks around it
decides, since Rust shadows (#353): a `let` and the names of its pattern, an `if let`, a `while
let`, a `match` arm, a `for`, a closure's parameters and the function's, wrapped one per line too,
`self` aside. In `let x = x.trim();` the right-hand `x` is the earlier binding, a nested `fn` sees
nothing of the function around it, and a binding on the cursor's own line counts without hiding.
A macro, a path, and a field a literal names with `w:` are no local. A binding the rules cannot
read (a macro that binds, a `let` pattern wrapped over lines) proves nothing: the search by name
stays. A TypeScript class header prettier wrapped is one header: a list of
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

PHP's `$this->word`, `self::word` and `static::word` are a member of the class the cursor is in:
the nearest `class`, `interface` or `enum` line above, indented less. Its methods, its properties
(promoted constructor parameters included), its constants and enum cases and the tags of its
docblock count; a call is a method, `->word` and `::$word` a property, `::WORD` a constant. When
the class does not declare the word, the traits its body `use`s come first, then the class it
`extends`, walked the same way up to eight levels: `open → BaseStorage::open (via $this:
ImageStorage)`. `parent::word` starts the walk one level up, `(via parent of ImageStorage)`. A
trait or a parent is the one its `use` import names, else the one of the file's `namespace`
(#351); one the project does not declare is read from its file in `vendor/`, and a member found
nowhere there is `no definition`, never another class's namesake. A cursor in an anonymous class
or in a trait's own body, a trait or a parent the project declares twice with no import to
choose, and a walk that finds nothing inside the project leave the word to the search by name.

A PHP class name, and `Class::word`, follow Composer's PSR-4 map (#351): the `autoload.psr-4` and
`autoload-dev.psr-4` of every `composer.json` above the file, read and never run: in a monorepo a
package's own file first, then the root's for what it does not map (#579). Where PHP
reads a class name (before `::`, after `new`, `extends`, `implements`, `instanceof` or `catch (`,
a type hint, a return type, the last part of a `use` line), the name resolves as PHP resolves it:
through the file's column-0 `use` (`use A\B\C;`, `use A\B\C as D;`), else a leading `\` spells it in
full, else the file's `namespace` goes in front of it. In a file of several `namespace` blocks
the `use` lines and the namespace are those of the block the name is in, and `<?php namespace X;`
on one line declares one (#579). A name a group `use A\{B, C}` binds is
left to the search by name, `use function` and `use const` bind no class, and a function or a
constant keeps the search by name, since PHP falls back to the global namespace for those. The map names the file, `App\Models\Song` in
`app/Models/Song.php`; when that file declares the class, `d` lands on it, `Song: via import
app/Models/Song.php`, or walks it for the member as `$this` is walked above: `query → Song::query
(via import app/Models/Song.php)`, and `toArray → AlbumResource::toArray (via AlbumResource)` for
a class of the file's own namespace, whichever other `AlbumResource` the project has. A mapped file
that is missing or declares something else (a classmap directory), and a member the walk finds
nowhere (Eloquent's `where`, which `__callStatic` forwards), leave the word to the search by name. A name the map does not cover is outside the project: its class in `vendor/`, in its
own namespace, is read first, `get → Arr::get (via import Illuminate/Support/Arr)` behind `use Illuminate\Support\Arr;`
whatever `get` the project has, and the search by name follows when `vendor/` has nothing. The
empty prefix `""` maps any name, so it covers only a name whose file is there.

PHP's receivers are typed too (#361): on `$x->word` and `$this->f->word`, `d` proves the class of
`$x` and of each property after it, up to six names in front of the word, then walks that class
as `$this` is walked above. `$x` is `$this`, or a variable whose every binding in its function
or closure, above the cursor, reads one class: a parameter with a class type, nullable
(`?Song $song`) or promoted, a header wrapped over several lines included; `$x = new T(…)`
(`new self` is the class around it); `$x = T::make(…)`, `$x = $this->make(…)` or `$x = make(…)`
whose declared return type is one class (`): self` is the class declaring it). A property is
typed by its declaration, `private SongRepository $songs;` or a promoted constructor parameter.
A class is the one the file's `use` import names, else the one of the file's `namespace`. The status line lists the links: `getRecentlyAdded → ArtistRepository::getRecentlyAdded
(via $this->artistRepository: ArtistRepository)`, `toArray → ScanInformation::toArray (via
ScanInformation::make(): self)`. A union or an intersection, `mixed`, `array` and the other
types of no single class, a `static` return type, docblocks (`@var`, `@return`, `@property`), a
`foreach` target, a value that is not the whole call (`T::make()->other()`), two bindings of
different classes, and a member the walk does not find leave the word to the search by name.

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

In Python a class declared outside the project is read as the project's are (#340): the import
that names it is followed to its one `class` line there, through the module's re-exports, and its
members and bases are read from that file and its own imports, relative ones against the
dependency's package: `self.assertEqual` in a `unittest.TestCase` subclass is
`assertEqual → TestCase.assertEqual (via self: T)` in `unittest/case.py`. An attribute no line
declares, such as the `objects` Django's metaclass makes, stays `no definition`, and a base that
cannot be read (a call, a compiled class) leaves the word to the rules below. In the other
languages a type declared outside the project, and in any language a link the rules cannot prove,
leaves the word to the search by name below. With two or more names in front of the word the status line says where the
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
function and bodiless signatures, Go `func (r *T) Name(`, Rust `fn name` directly in an `impl` or
a `trait`. They are collected from the project and from the standard library and dependencies,
where TypeScript is read from its `.d.ts` files only. Rust drops what the cursor cannot reach: an
inherent method without `pub` outside its module, a `pub(crate)` one outside its crate, a crate
`Cargo.lock` does not lead to from the cursor's, a registry copy of a package of the project, a
crate's tests, a trait outside the project that is not `pub`, and a method or a trait of the
standard library with no `#[stable]` or `#[unstable]`, which is its own. A method a macro writes
counts, whatever its `pub` says. The traits' methods come first,
and when every candidate is the method of one trait or of an `impl` of it, `d` jumps to the
trait's, `via trait Clone`.
So is every field of that name in the project, one row per type, on the line a proven receiver
would land on: Python `name: T` or `name = …` in a class body and `self.name = …` in a method,
TypeScript members and constructor parameters behind a modifier and `this.name = …`, the
`@property` lines of a JavaScript file's `@typedef {Object}`, Go struct fields and embedded structs. A local, the key of a dict or an object literal and a line of a `var`
block are no field, and a name several types declare, such as `id`, is a picker rather than a
jump. The field lines are searched apart from the methods, and when they fill the search the count
says `+` and a single candidate is offered rather than jumped to. Fields outside the project are
not collected: there a field name is every `name: string;` of every `.d.ts`. In Python they are
looked for all the same, in the same pass as the methods: when the project has no candidate and
one method outside is all there is, a field of the name outside makes that method one candidate
of `1+`, offered and not jumped to (`m.return_value` on a `mock.Mock` is a field of
`unittest/mock.py`, not a method of some other package). A project with no
such method or field has the word at its top level instead — `x` was a class or a
namespace — and the usual declarations answer. One candidate jumps; several open the picker, the
project's first. A bare `self.word` or `this.word` whose class, or a class it extends, cannot be
read gets the project's declarations of that name, fields included, and nothing outside the
project. In Python, when the class is read and does not declare the word, but its ancestry goes
outside the project — Django's `TestCase` behind `self.client` — the answer lies there or in a
project class extending it that sets the word on `self`: only those subclasses' declarations are
offered, and none is `no definition`, never another project class's `client`. A base that cannot
be read (a call such as `six.with_metaclass(…)`, a name nothing binds, a `*` import) leaves the
search by name as it was. Likewise `User.objects` with `User` imported from a module outside is a
member of `User` in that module, never a top-level `objects` there or anywhere else, when the
module declares the class `User` or imports it. A value the module holds instead, such as
Django's `settings`, whose `__getattr__` reads the project's settings module, finds its member
only among the project's module-level `NAME = …`, else among those of the module's own package
(Django's defaults in `django/conf/global_settings.py`): never a namesake elsewhere outside the
project, nor a method or a field of a project class. In Ruby a member on a value, `logger.info` or
`x&.each`, is never jumped to, whatever is read outside the project: one method of that name is
offered in a picker of one row, `info: by name, 1 match`, since a value whose type is not known
may answer with a method declared nowhere, an ActiveRecord column's. `Const.meth`, `self.meth` and
a bare call still jump.
PHP reaches a member with `->` and `?->`, on the arrow's line or at the start of the next one
when a chain is broken before its arrows; its `.` concatenates, so `$a.foo()` calls the function
`foo`. Behind the arrow a call, `$x->name(`, is a method — `function name(` indented, or a
`@method` tag of a class's docblock — and anything else a property: `$name` behind a modifier, a
promoted constructor parameter or a `@property` tag. The project's come first, then `vendor/`'s,
and neither a local `$name = …` nor a function, a class or a constant of that name is ever the
answer, `$this->name` included: with no member of the name, `d` says `no definition`.

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
| TypeScript / JavaScript | `function`, `class`, `interface`, `type`, `enum`, `namespace`, `const`/`let`/`var` (so arrow functions assigned to a name), class and object-literal methods, properties holding a function, a method signature with a return type and no body (`find(id: string): User;` in an interface, an abstract class, an overload or a `.d.ts`), behind `export`/`default`/`declare`/`async` and the member modifiers; behind a dot, a field: a member `name: T;` or `name = …`, a constructor parameter behind a modifier, `this.name = …`. Destructuring and parameters have no rule. | every `.ts`, `.tsx`, `.js`, `.jsx` and friend, and every `.vue`, `.svelte` and `.astro` component: they search each other |
| Rust | `fn`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`, `mod`, `macro_rules!`, behind any `pub(..)`/`async`/`unsafe`/`const`/`extern`/`default` prefix, and an enum variant for `Enum::Variant` or behind a glob `use`. `impl` blocks count as uses. Behind a dot with no `(` after the word, a field: `name: T` in a `struct`, a `union` or a variant `Name {`, the project's, else the `pub` ones outside it. In an attribute, a derive is a `pub macro W` or a `#[proc_macro_derive(W`, the attribute's name a `pub macro W` or a `pub fn W` under `#[proc_macro_attribute]`, and a `cfg` predicate or a compiler attribute (`allow`, `repr`, `inline`, …) nothing. A `let` and a parameter are locals of their block, read by the scope walk, never found by name. | every `.rs` file |
| Java | `class`, `interface`, `enum`, `record`, `@interface`; an enum constant behind its enum, `Offer.CUT`; a method, an abstract or interface method and a field, told from a call by the return type before the name — a primitive, or a name with a capital in it, as Java writes its types; a constructor, behind at least one modifier, since a bare `Name(x) {` is a call, its parameters on one line or wrapped over several (`Name(` at the end of the line); a record's components, on one line or wrapped, each both a field and its accessor `name()`. Annotations and modifiers may stand in front of any of them. A local (a declaration whose block is no type's body) answers only below it in its own block, and a `private` declaration only in its file. `Type::m` is a qualified name as `Type.m` is, and `this::m` or `this.m` is `m` of the class around the cursor. A Lombok accessor no method declares, `getTitle`, `isActive` or `setTitle`, is the field `title` (or `active`, `isActive`) of a file that imports `lombok.`, when the field carries `@Getter` or `@Setter`, or its class `@Data`, `@Value` (no setters) or `@Getter`, or `@Data` or `@Setter`; not a `static` one or one marked `AccessLevel.NONE` (#381). Behind a receiver of a known type or a qualifier naming the class, `User::getTitle`, it is that class's or a class's it extends; otherwise every such field is offered, never jumped to. A call `name(` is a method or a constructor, never a variable, and `values()` or `valueOf(` an enum's, the one around the cursor or the one in front; behind `new` a class that declares constructors offers them, and elsewhere its `class` line; the number of arguments keeps the overloads and constructors it fits (a varargs `...` takes any more), unless they hold a `<` or none fits. A bare name is looked for in the scope around the cursor first: a parameter, a lambda's parameter, a loop variable or a local of the blocks around it (`local`), then a member of the classes around it, innermost first (`via` the class), then of the class the innermost one extends, when the project declares it once. | every `.java`, `.kt`, `.kts` and Scala file: they search each other |
| Kotlin | `fun` and `val`/`var` (an extension is named by its receiver: `fun Topic.asExternalModel()` is `Topic.asExternalModel`), `class`, `interface`, `object`, `enum class` and a constant behind it, `Speed.FAST`, `typealias`, behind `private`/`open`/`data`/`sealed`/`suspend`/`override` and the rest. A local (a declaration whose block is no type's body) answers only below it in its own block, and a `private` declaration only in its file. A bare name is looked for in the scope around the cursor first, as in Java: parameters, lambda parameters (`{ a, x ->`), loop variables and locals, then the members of the classes around it, a `companion object`'s and the primary constructor's properties included, then of the supertype the project declares once. The `val` and `var` of a primary constructor on its class's line are properties. A word between two operands with no `.` in front and no `(` behind, `alias(x) apply false`, `a to b`, is an infix call: only an `infix fun` answers it. `it` and the implicit receivers of `with` and `apply` are not read. | every `.java`, `.kt`, `.kts` and Scala file: they search each other |
| Scala | `class`, `case class`, `trait`, `object`, `case object`, `package object`, `enum` and its cases (`case Red, Green`, `case Circle(r: Double)`, `case Mercury extends Planet(…)`), `def` (a Scala 3 extension's on its `extension` line), `val`/`var`/`lazy val`, `type`/`opaque type`, a named `given`, a `val` or `var` among a class's parameters and every parameter of a `case class` on its line, behind annotations and Java's, Kotlin's and Scala's modifiers (`implicit`, `lazy`, `case`, `opaque`, `private[shop]` and the rest). A name in backticks is the word between them. A match case (`case Invoice(id, _) =>`), Java's `case RED:`, an anonymous `given`, a method's parameter and an operator (`def +`) declare nothing. Imports narrow nothing: an `import a.{B, C => D}` is looked for by name (#416) | every `.scala`, `.sc`, `.sbt` and `.mill` file, with the `.java`, `.kt` and `.kts` ones |
| Ruby | `def`, `def self.name`, `class`, `module`, an assignment: a constant anywhere, an `@ivar` or `@@cvar` (a word of its own, sigil included) only in its class and a reopening of it, a local only in its own method or block, and none of them behind a dot (#383). A parameter of the `def` or the block the cursor is in, and a local assigned above it there (`x =`, `x ||=`, `a, x = …`, `rescue => x`, `for x in`), is the answer, `local`; scopes are read by indentation, and a block sees its `def`'s names. A call with no receiver that no local names, and `self.name`, is looked up in the class around the cursor first, in any file that reopens it, then in the modules it `include`s (`extend`s, where `self` is the class) and its superclasses, `via Klass`; inside `class << self` only the class's own methods count (#365), `attr_accessor`/`attr_reader`/`attr_writer`, `alias`/`alias_method`. A trailing `?` or `!` is part of the word, so `d` on `empty?` finds `def empty?` and `d` on `empty` does not; `x.name = v` asks for the setter `name=`, which `def name=` and `attr_writer`/`attr_accessor :name` declare. `class A::B` declares `B` inside `A`, and `A::B` in code is a path, as `Outer.find` is. `Const.meth` is a class method: `def self.meth`, a `def` inside `class << self`, one of an `extend self` or `module_function` module, or of the `class_methods do` / `module ClassMethods` of a concern the class includes; `Const.new` finds its `initialize`. An instance method of the class is no answer, and when the class declares none the answer is outside the project, by name (#369), never a method of another project class. The Rails DSL declares the name it is given (#374): `has_many`, `has_one`, `belongs_to`, `has_and_belongs_to_many`, `scope`, `has_one_attached`, `has_many_attached`, `has_attached_file`, `attribute`, `alias_attribute`, `enum :name` and `enum name:`, every name after the store's in `store_accessor`, and the names of a `delegate` of one line with no `prefix:` (other than `prefix: false`), and Rails' class-level accessors, `mattr_accessor`, `cattr_accessor`, their readers and writers, and `config_accessor`, which are class methods (#369). A `scope` is a class method, of the class or of the `included do` block of a concern it includes. In `db/schema.rb`, `t.string "name"` declares a column, named by its table in the picker; a migration's `t.` lines and `db/structure.sql` are not read. A member on a value found by name is offered, not jumped to (#390). | every `.rb`, `.rake`, `.gemspec`, `.podspec`, `.rbi`, `.ru` file and `Rakefile`, `Gemfile`, `Vagrantfile` and friends |
| C / C++ | a function, a prototype and an out-of-line method (`Type::name(`) in column zero, where the languages have no statements, so a call is never one — the return type may sit on the line above, as GNU style writes it; a method or a function indented, when its body opens on the line; `struct`, `class`, `union`, `enum`, `enum class`, `namespace`, behind a template head, a storage specifier and an attribute or export macro (`struct __attribute__ ((__packed__)) sdshdr8`, `class FMT_API name`), a template specialization included; `typedef` in every form, `using x =`, `#define` (function-like too), a global. One definition beats its prototypes (or a variable's `extern` declarations) when they take the same parameters, and the status says what was set aside (`add: by name, 1 definition, 1 prototype`); overloads and `#if` / `#else` variants stay a picker, and on the definition the prototype is offered. What another source file keeps to itself — a `static` at file scope, a `#define`, an unnamed `namespace {` — is not offered unless the file on screen `#include`s it, while a `static` of the file on screen is the answer there. A `#define X` under `#ifndef X` stands in only where nothing else declares `X`: the project's other declarations, else the system's, come first. A type lands on its body, not on its forward declarations and constructors, and `struct Outer::Inner {` declares `Inner`. Behind `->` or `.` a word is a member: a field of a `struct`, `union` or `class` body (`T name;`, `T *a, *name;`, a bit-field, an array, `T (*name)(…);`, C++'s `T name = init;` and `T name{init};`, the `} name;` of a nested anonymous body), or when called a method whose body opens on its line, an out-of-line `R Type::name(` or a function-pointer field; no function, macro, type or global is offered there, and outside the project only fields are searched. A name in a C++ constructor's initializer list, `: filename_(name)`, is a field of that class first, and so is a bare name inside a method of the class. A parameter or a local is the function's own (`local`) and hides every other declaration of the name: the innermost block above the cursor that declares it, a closed block not counted, then what a `for`, `if`, `while` or `switch` head binds for its body, then the parameters, a lambda's reading on into the function around it. A word followed by `->` or `.` is a value, never a struct, `typedef` or `using` name. An enum constant is a name on a line of `NAME,` or `NAME = expr,` directly inside a body whose opener reads `enum` (`enum class`, `typedef enum`), or in a one-line `enum X { A, B };`: the line is the same in an initializer list, but its opener, `= {`, is not. A member function declared with no body directly inside a `class` or `struct` body (`virtual … = 0;`, `… override;`, `T name(…) const;`, the first line of a parameter list that wraps) is a declaration, while the same line in a function body is a local object; in a picker a member declared so and defined out of line, `R X::name(`, is one row, the definition, and on that definition `d` offers X's declarations of the name and nothing else. A template parameter has no rule. | every `.c`, `.h`, `.cc`, `.cpp`, `.cxx`, `.hpp`, `.hh`, `.hxx`, `.m` and `.mm` file: they search each other |
| Objective-C | every rule of C and C++ above, and (#417): a class, `@interface Name : Super <P>`, `@interface Name <P>` or `@interface Root`, and a protocol, `@protocol Name <P>` or alone on its line — a category `@interface NSString (Slug)`, a class extension `@interface Name ()`, `@implementation Name`, `@class Name;` and `@protocol Name;` declare nothing; a method in column zero by any part of its selector, `- (User *)findUser:(NSString *)name inContext:(Context *)ctx;` declaring `findUser` and `inContext`, and so does the line of its definition, offered beside the declaration as a C prototype is beside its definition; `+ (instancetype)shared`; a property, `@property (nonatomic, copy) NSString *baseURL;` and a block's `void (^completion)(NSError *error);`, which `self.baseURL` reaches as `x.field` reaches a field; `typedef NS_ENUM(NSInteger, Status)` and `NS_OPTIONS`, `NS_CLOSED_ENUM`, `NS_ERROR_ENUM`, by the name after the comma; a block type, `typedef void (^Done)(NSError *error);`, and a `typedef` an `NS_`, `CF_` or `API_` macro follows. Behind a dot, `x.word` is a property, the getter one names (`getter=isRunning`) or a method that takes no argument, which dot syntax calls; a property another `.m` file declares in a class extension of its own is that file's and is not offered. A method's parameters and what `for (T *x in xs)` binds are locals, as a C function's are. An Objective-C line declares for an Objective-C file only: a C or C++ file finds what it found before. A name a class or a protocol declares lands there, never on a C line read through a comment; of a class and a protocol of one name, `NSObject`, the protocol is the one inside `<…>`. A message send, a `@selector(…)`, a dot access, an enum constant of an `NS_ENUM` and an instance variable declare nothing. | every file of the C kind |
| C# | `class`, `struct`, `interface`, `enum`, `record`, `record class`, `record struct`, `delegate`, past the generic parameters they declare and behind `[Attribute]` lists and any modifiers (`public sealed partial class Foo<T>`); a `namespace`, under its last part; a `using x =` alias; a constructor, behind at least one access modifier, since a bare `Invoice(n)` is a call, with its body on its line or its parameters wrapped onto the lines below; and a method, a property, an event, a field or a local, told from a call by the type before the name — a predefined one, `var`, or a name with a capital in it, as C# names its types — so `public int X { get; }`, `public string Name => _name;` and `int IComparable.CompareTo(o)` all count. An enum member has no rule of its own: `Open,` in an `enum` body and in a collection initialiser are the same line. Behind the name of an `enum` the project declares in a namespace the file sees — its own, a `using`, a `global using` of its `.csproj`, or the path written out — `Offer.Cut` lands on the member in the enum's body, `via Offer`. The search by name offers no private member of another type (a part of a `partial` type the file declares is its own), no local or local function of another method, and nothing behind a dot for a local; a member behind a type name the project declares in no form, `Task.Delay`, is `no definition`, since the framework's assemblies are not read. A parameter, a lambda's parameter, a loop variable or a local lands on its binding in the method the cursor is in, `local`, and its namesakes elsewhere are not offered. Where only a type can stand (an identifier after it, `new`, `typeof(`, `is`, `as`, a cast, a generic argument, a base in a type's header) only a type, a delegate or an alias counts, and a constructor yields to its type in its file wherever the name is used. A bare name in a type is what that type, a `partial` part of it, the bases it names, walked up, or a type around it declares, `via OrderViewModel`, before the search by name, which then offers no member of any other type unless a `using static` may bring it in. A segment of a `using` or `namespace` line, and of a `global::` path that one answers, is the project's `namespace` of that dotted name or one inside it, and nothing else. A method whose parameters, defaults and `params` counted, cannot take the call's number of arguments is not offered. | every `.cs` and `.csx` file of the projects the file compiles against: the nearest `.csproj` above it and the `<ProjectReference>`s of that one and of every `Directory.Build.props` above it, followed transitively, plus every file under no `.csproj`; the whole repository when that cannot be told (no `.csproj` above, two in one directory, a `<Compile Include>` outside its directory, a shared project's `.projitems`, a reference not read) |
| Swift | `class`, `struct`, `enum`, `protocol`, `actor`, `typealias`, `associatedtype`, `extension Type` — only when the project declares no `Type`: it is looked for among the dependencies SwiftPM checked out first, and when none declares it either (Foundation and the standard library ship no source) its extensions are offered in the picker, never jumped to (#371) — `func` past its generic parameters, `init`, `init?`, `subscript` and `deinit`, `let` / `var` (one inside a function only for a bare word in that same function, never behind a `.`), and an `enum` case, alone or among several on a line, with the associated or raw value it carries. All of them behind their `@attributes` and any modifiers (`public final override class func`, `private(set)` included), and a backticked name counts. A `case .open:` or `case let .open(x):` of a `switch` is a pattern, not a declaration. A name bound in the function the cursor is in is found first (#366), walking out over the blocks around the cursor, the innermost binding above it winning: a `let` / `var` (`let a = 1, b = 2`, `let (a, b) = t`) or a `guard let` statement of each block; what a block's header binds, `if let` / `while let` / `if case let` (each clause), `for x in` and `for (i, x) in`, `catch let e` and a bare `catch` for its implicit `error`, a closure's `{ a, b in` / `{ (a: T) in` / `{ [weak self] a in`, a `switch` case's `case let .x(a):` / `case .x(let a):`; the parameters of the `func`, `init` or `subscript`, over the lines its header wraps, and on out past a nested function's header into the function around it (#564); a generic parameter of a `func`, `init`, `subscript` or `typealias` header around the cursor, or of the innermost type's, `<Value: Sendable>`, lands on that header's line (#375), but a `where` clause binds nothing. Past a function inside another, a closure or an accessor, the walk proves no local, since it reads no local `func` or type. The shorthand `if let x {` / `guard let x else` rebinds an outer `x`, so the walk goes on past it; a type's body and the top of the file bind no local, and a pattern the walk cannot read (a nested tuple) sends the word to the search by name. A member's kind narrows it (#380): an implicit member, `.word` behind `(`, `,`, `[`, `:`, `=`, an operator, `return`, `case` or at a line's start, is one of the `enum` cases and `static` / `class` members of the name, and a `case` pattern's one of the cases; `Type.word` on a type the project declares is its case or `static` member rather than an instance one; a bare word in a type's body is a member of that type or its extensions, then of the class its header extends first, `(via self: Type)`, unless a local or parameter hides it or an `extension … where` constrains the type. When none of them declares the word, the search by name decides, and it offers nothing the compiler cannot see from the cursor (#375): outside a test target, a test target's declaration (the `path:` of each `.testTarget(` of the root `Package.swift`, else `Tests/<name>`; `Tests/` when there is no `Package.swift`); another file's `private` or `fileprivate`; a type declared inside a function, outside that function; and for a bare `Name`, a class, struct, enum or actor nested as `Outer.Name` outside `Outer`'s body, its extensions and a class whose header names `Outer` first. `Outer.Name` written out is found as before. | every `.swift` file |
| PHP | `function` (`&` included), `class`, `interface`, `trait`, `enum`, a `const` with its type or without (`const int LIMIT`) and a `define('X', …)`, an `enum` case, a property with the type it carries, and a constructor parameter promoted to one — all behind their `#[Attribute]`s and modifiers (`final public static function`) — plus an assignment that opens a line (`$x =`, `.=`, `??=`, `+=`), and the `@property`, `@property-read`, `@property-write` and `@method` tags of the docblock right above a class, which declare a member of it (`Song::title`). A `namespace` line answers only a segment of a qualified name, a word a `\` follows, and only when it declares the namespace written up to that word (`Illuminate\Support` for `Support` in `use Illuminate\Support\Facades\Route;`, relative to the file's namespace outside a `use` unless it starts with `\`); on a `namespace` line's last part the other files of the namespace are offered. A `case X:` of a `switch`, a `$key => $value` pair, `$rows['x'] =` and `$this->name = …`, which writes to a property declared elsewhere, are not declarations. A `$variable` is looked for in its own function or closure only (#464): its parameters and `use` list, and above the cursor a plain `=`, a `foreach` or `catch` target, a destructuring, a `global` or `static`; at the top of a file, the file's own lines. `.=`, `+=` and `??=` read the variable first and bind nothing there. | every `.php` and `.phtml` file |
| Lua | `function name(`, `local function name(`, `function M.name(`, `function M:name(` and the longer `function a.b.name(`; a function literal bound to a name (`M.name = function(`, `name = function(` in a table of handlers); `local name`, one of several on the line included. A field holding anything else has no rule: `limit = 10` in a table constructor and a re-assignment inside a body are the same line, and the language has no keyword to tell them apart. | every `.lua` file |
| Elixir | every `def` form — `def`, `defp`, `defmacro`, `defmacrop`, `defguard`, `defguardp`, `defdelegate` — written `def name(x) do`, `def name do` or `def name, do: x`, a trailing `?` or `!` included; `defmodule` and `defprotocol` under the namespace they are written with, by their last part, so `defmodule MyApp.Repo` declares `MyApp.Repo` and nothing called `MyApp`; a `defstruct` field, atom list or keyword form, on the `defstruct` line itself — a field on a continuation line of a struct written over several lines has no rule, since that line is the shape of any keyword list; a module attribute where it is given a value (`@timeout 5_000`). Several clauses of one function are several declarations and all are offered. `@spec`, `@type` and the rest of the attributes the language and the libraries everyone uses own — ExUnit's `@tag`, Mix's `@shortdoc` — are directives: `@spec parse(t) :: t` is no declaration of `parse`, and `d` on one of those names has nothing to find. That is a list of known names, which is all a line pattern can have: any library may define an attribute, and `@tag :slow` and `@timeout 5_000` are the same line. `defimpl` declares the module `Protocol.Type`, where neither half is a name of its own, as a Rust `impl` is not. | every `.ex` and `.exs` file |
| Zig | `fn name(`, behind `pub`, `export`, `extern "c"`, `inline`, `noinline`; `const` and `var`, which is how the language declares a type (`const Ledger = struct {`, `const Status = enum {`, `const Value = union(enum) {`), an import, a constant and a local alike, `threadlocal` and `comptime` included. A struct field (`total: u32,`) has no rule, as a C field has none: it is the shape of a value in a struct literal. Neither has a `test`: a word inside its description declares nothing, so `d` can never land there — `D` lists the tests instead. | every `.zig` file |
| Protocol Buffers | `message`, `enum`, `service` and `oneof`, a nested message included; `rpc Name(`, braces or not, `stream` arguments too; an enum value, `NAME = 1;`, with no type before the name; a field, `string id = 1;`, `repeated Order orders = 2;`, `map<string, int32> counts = 3;`, `optional`, `required` and a qualified type included. A field's type, an rpc's argument and return types and the message an `extend` adds to are uses, as a Rust `impl` is; an `option`, a `reserved` list and a name inside an import's string declare nothing. The text format (`.textproto`, `.pbtxt`) is data and has no rules. | every `.proto` file |
| Shell | `name()` and `function name`, an assignment behind `export`/`declare`/`local`/`readonly`/`typeset` (or bare, and `+=`), `alias` | every `.sh`, `.bash`, `.zsh`, `.ksh` and shell dotfile (`.bashrc`, `.zshrc`, `.profile` and friends) |
| PowerShell | `function` and `filter`, behind a scope (`function global:Get-ShopUser`), under the whole `Verb-Noun` name; `class` and `enum`; inside a class a property (`[string] $Name`, `hidden [int]$Count = 0`), a method (`[decimal] Total() {`, `static [Invoice] Parse(…) {`) and a constructor (`Invoice([string] $id) {`); an enum member on a line directly inside an `enum`; an assignment that opens a line (`$Config = @{`, `$script:Cache = @{}`, `[string]$Name = 'x'`, `$Count += 1`); `Set-Alias` and `New-Alias`. Names ignore case, as PowerShell does. A `$variable` is only a variable or a property, a bare word never one (`$tariff` declares no `Tariff`), and a constructor counts only where its class is built (`[Tariff]::new(`). A parameter of a `param(` block in the blocks around the cursor, a function's, a script block's or the script's, or of a `function Name($a)` header, is `local` and never looked for in another file. A call, a named argument, a hashtable key, a property or element write, a comparison and splatting declare nothing; nor does a line in comment-based help (`<# … #>`) or a here-string. `d` on the path of a dot-source (`. $PSScriptRoot/helpers.ps1`), an `Import-Module ./Shop/Users.psm1` or a `using module` opens that file. | every `.ps1`, `.psm1` and `.psd1` file |
| Dart | `class` behind `abstract`, `sealed`, `base`, `final`, `interface` and `mixin`; `mixin`, a named `extension … on`, `extension type`, `enum` and `typedef`, the new form and the old (`typedef void Callback(int code);`); in column zero, where Dart has declarations and directives only, a function (`String formatPrice(int cents) =>`, `main() {`); indented, a method told from a call by the return type before its name — a primitive (`void`, `int`, `double`, `num`, `bool`, `dynamic`) or a name with a capital, with its generics and `?` — as Java's is; a getter and a setter; a constructor directly inside its class, an enum or an extension type, behind `const`, `factory` or `external` or with parameters that open with `this.`, `super.`, `{`, `[` or a type and a name, or followed by the `:` of an initializer list — `User.fromJson` under the name after the dot, the class's own `User(` only where the class is built, `User(…)`; a variable or a field behind `final`, `const`, `var` or `late` (`static`, `external` and `covariant` too), with or without its type, or with a type alone (`String? label;`), outside a parameter list wrapped over lines; an `enum` value on the `enum` line or on a line directly inside the `enum`. A call statement (`Navigator.push(context, route);`, `return Foo(x);`, `throw StateError('x');`), a named constructor's call with no arguments (`User.empty();`, the shape of a declaration with none), a method with no return type (`build(context) {`), an unnamed `extension on String`, a parameter and a pattern of a `switch` declare nothing, nor does a line inside a `'''` or `"""` string or a `/* */` comment. `$` is part of a name, at its start too (`_$UserFromJson`, `$UserCopyWith`), save in a string, where `'$name'` interpolates `name`. | every `.dart` file |
| CMake | `function(name …)` and `macro(name …)`, whose name ignores case as every command's does, in the definition (`FUNCTION(`) and in a call (`SHOP_ADD_LIBRARY(…)` calls `shop_add_library`); a variable of `set(NAME …)`, a cache entry included, and of `option(NAME …)`; a target of `add_library`, `add_executable` and `add_custom_target`, an alias (`add_library(Shop::core ALIAS shop_core)`) and an imported one (`add_library(Foo::foo UNKNOWN IMPORTED)`) included, for a use in `target_link_libraries`, `add_dependencies` or a generator expression (`$<TARGET_FILE:shop_app>`). A variable and a target keep their case. A call, a variable's use (`${X}`, `if(X)`), a target's use, a keyword (`PRIVATE`, `STATIC`), `set_target_properties(` and `set_property(`, `list(APPEND X …)`, `unset(X)`, `set(ENV{X} …)` and `endfunction(name)` declare nothing, nor does a line inside a bracket argument (`[[…]]`, `[=[…]=]`), a bracket comment (`#[[…]]`, `#[==[…]==]`) or a quoted argument over lines; a target a function creates through `${name}` has no line that spells it, and `d` finds none. `d` on the argument of `include(ShopHelpers)` opens the project's `ShopHelpers.cmake`, else CMake's own module of that name; `include(cmake/warnings.cmake)` is a path from the file's directory; `add_subdirectory(app)` opens `app/CMakeLists.txt`; `find_package(Boost)` opens `FindBoost.cmake`, `BoostConfig.cmake` or `boost-config.cmake`, the project's or else one outside it. | every `.cmake` file and `CMakeLists.txt` |
| Nix | an attribute binding, `mkService = …;` (a name, then `=` and not `==`): the last part of a dotted path (`services.nginx.enable = true;` binds `enable`), a quoted name (`"my-attr" = …;`), and each name of `inherit name port;` and `inherit (pkgs) hello;`. A binding between a `let` and its `in`, on one line or over several, is local: it is found only in the file the cursor is in, in the `let` blocks whose bindings or body hold the cursor's line, innermost first, `api: local`, and never over the project, where every other file has an `api` or a `cfg`. So is a parameter: the names of a set pattern, `{ config, pkgs, lib, ... }:` (a file's header or a lambda's, on one line or over several, `args@{ … }` and `{ … }@args` included), and each plain `x:` (`final: prev:`), for the body of that lambda, `pkgs: local`, whatever binding of the name another file holds. A body ends where the bracket around it closes, at the `;` that ends its binding (not one of `with x;` or `assert x;`), or at the `then`, `else` or `in` around it, so a sibling attribute's `let` or a lambda closed on a line above binds nothing below it. A use (`pkgs.hello`, `config.services.nginx.enable`, `with pkgs;`), a comparison (`==`), a default in a parameter set (`port ? 8080`, a parameter), a name inside `${…}`, a `#` comment, a one-line string and a line inside an indented string (`'' … ''`, with its `'''`, `''$` and `''\` escapes), a `"…"` over lines or a `/* */` comment declare nothing; `//` is the update operator, no comment. `d` on a path (`./nginx.nix`, `../lib`) opens that file, or a directory's `default.nix`, from the file that writes it; `<nixpkgs>` is not followed. | every `.nix` file |
| SQL | `CREATE` of a table, view, index, function, procedure, trigger, type, schema, sequence, domain, extension, database, role or user, behind `OR REPLACE`, `TEMP`, `UNLOGGED`, `MATERIALIZED`, `UNIQUE` and `IF NOT EXISTS`, schema-qualified or quoted; a `WITH … AS (` common table expression. Keywords ignore case. Columns have no rule. | every `.sql`, `.psql`, `.pgsql`, `.mysql`, `.ddl` and `.dml` file |
| Makefile, `*.mk` | a target, also one of several before the colon; a variable, outside a recipe; a variable set only by `+=` or for one target (`release: VERSION := 1.0`), when nothing assigns it plainly | every Makefile |
| Terraform | the block behind `var.x`, `module.x`, `local.x`, `data.T.N`, `T.N`; a bare name, as in `.tfvars`, is any block with that label | `.tf` files in the same directory |
| Dockerfile | the `FROM … AS name` stage | the same file |
| YAML | the `&name` anchor, a key that opens a block (compose services, CI jobs) | the same file |
| Markdown (`.md`, `.markdown`, `.mdx`) | no declarations: `d` follows what the cursor stands on. A link, on its text or its target: `[text](target)`, a reference `[text][label]`, `[label][]` or `[label]` through its `[label]: target` definition (and on that line), an `<a href>`; not an image. The target is a path, percent-decoded, from the file's directory or, after a `/`, from the root; `#anchor` is the heading with that GitHub anchor (the second of a name `-1`) or an `<a id>` / `<a name>`, in that file or this one; `#L12` and `#L12-L20` are a line. A missing file or heading, a directory and a URL say so. A code span naming a file of the project, from the root or from here, opens it, at `:line` when it has one; a bare name several files carry is a picker of them. Nothing in a fenced block, an HTML comment or the front matter is followed. | the file the link names |
| CSS, SCSS, Sass, Less | A class or an id under the cursor, in a selector, an `@extend .btn;` or a Less mixin call `.bordered();`, is looked up as from an attribute (below). `var(--brand)` is every `--brand:` declaration (`:root`, a theme's override) and an `@property --brand`; SCSS's `$primary` is `$primary:` at the start of a line, `@include name` an `@mixin name`, a call `tint-color(…)` in a value an `@function tint-color`, `@extend %shared;` a `%shared {` placeholder; Less's `@brand` is `@brand:`; a name in an `animation` or `animation-name` value is `@keyframes name`. A namespace of `@use 'variables' as v` (or the module's own name, `@use 'mixins'` giving `mixins`) narrows `v.$primary` to that module's file, `via import`; `as *` and `@import` bind none, and the name is found by name; a variable, a mixin or a function the project does not declare is then looked for in the `node_modules` of the file's directories up to the root: Sass names from a `.scss` or `.sass` file, Less variables from a `.less` file, never a CSS built-in function such as `rgb(`, nor a name behind a namespace no project file resolves, `sass:map`'s `map.get` among them. A map key, a value's `.5em` or `url(i.svg#check)`, and a word in a comment declare nothing. `d` on the path of `@import`, `@use` or `@forward` opens the file: beside the importing one, Sass trying `_x.scss`, `x.scss`, `_x.sass`, `x.sass`, `x.css`, `x/_index.scss`, `x/index.scss`, Less adding `.less`, then in the `node_modules` of its directories (webpack's `~` too); `sass:math` and the other built-in modules have no source, and a URL is not followed. | every `.css`, `.scss`, `.sass` and `.less` file and the `<style>` blocks of `.html`, `.vue`, `.svelte` and `.astro` files, by kind of name |
| HTML (`.html`, `.htm`), and a class or an id of JSX, Vue, Svelte and Astro | A word of a `class` or `className` value (`className={'a b'}`, Svelte's `class:active`) is the rules that style that class: a rule's selector is the text before its `{`, over a list of lines ending in `,`, and the class counts in its last compound, the element it styles (`.btn {`, `.btn:hover,`, `.card > .btn {`, `a.btn.active {`), not before a combinator (`.btn .icon {`) or inside `:not(…)`; in SCSS and Less an `&-primary` or `&__title` composes the name with the rule it sits in, walked up through `&` rules. A `.sass` rule is a selector line with the indented lines under it as its body, `&` composing as in SCSS. `styles.container` behind `import styles from './Button.module.css'` (`.module.scss`, `.module.sass`, `.module.less` too), or a name a named import takes from one, is `.container` in that file only, `via import`, when `styles` is the import itself: `theme.styles.x`, or a parameter or a local named `styles`, is TypeScript's member. An `id` value is the rules whose last compound holds `#id`. A class or an id is never looked for in `node_modules`. In JSX the attribute has no blank around its `=`, and a `class` or an `id` stands in a tag or first on a line of its attributes, so `id = "x"` stays an assignment. In an HTML file, `d` on the path of a `href` or a `src` opens the file, from the file's directory or, after a `/`, the project file whose path ends with it, at the element whose `id` or `name` its `#fragment` is; `href="#main"` is that element in the same file; a URL is not followed. A `<style>` block is read as a stylesheet, and nothing else of the file declares anything: an inline `<script>` is not read as JavaScript. | as for a stylesheet; a path, the file it names |
| GraphQL | `type` (behind `implements` and directives), `interface`, `input`, `enum`, `union`, `scalar`, `directive @name`, `fragment` (for a `...spread`), a named `query`, `mutation` or `subscription`, each at the start of its line; a field, one whose arguments wrap included, and an enum value, on a line directly inside a `type`, an `interface`, an `input` or an `enum` (an `extend` of one too), the nearest line above indented less. `extend type X` is a use of `X`, as a Rust `impl` is; a selection or an alias in an operation, an argument, a `$variable` and a line of a `"""` description declare nothing. `d` on the path of `#import "./parts.graphql"` opens that file, relative to the importing one. | every `.graphql`, `.graphqls` and `.gql` file |

A Vue, Svelte or Astro component is TypeScript in its script only: the lines inside each
`<script …>` … `</script>` block, the tags on lines of their own, and in Astro the frontmatter
between a `---` first line and the next `---`. The template, the `<style>` block and HTML comments
declare nothing, and the rules read them as blank lines; `D` lists the script's declarations. An
import of a `.vue`, `.svelte` or `.astro` file names that file, by a relative path or a
`tsconfig.json` alias: a default import lands on the component's `export default` line when its
script has one (`export default defineComponent({`), else on its first line, `UserCard: module
src/components/UserCard.vue`. A name in the template (`{{ label }}`, `@click="save"`, Svelte's
`{label}`) is looked up as from the script. A name the template binds is a local of the file:
Vue's `v-for`, `v-slot` and `#slot="{ item }"`, Svelte's `{#each … as item, i}`, `{:then value}`,
`{:catch error}`, `let:item`, `{@const total = …}` and the parameters of `{#snippet row(item)}`;
several lines binding it are a picker, and so is a binding of the same name at the top of the
script, since the element a binding is scoped to is not read. A tag no import binds, `<UserCard>`
or `<user-card>` (a global or auto-imported component), finds the component file of that name, by
name, or else the script's own `const` of it (`defineAsyncComponent`). A word in the `<style>`
block names nothing.

In Makefiles, Terraform, Dockerfiles, YAML, PowerShell, HTML and stylesheets a `-` is part of the
word under the cursor, and so it is inside a `class`, `className` or `id` value of any file; `d` in
Terraform reads the whole dotted address, so it works from anywhere in `aws_s3_bucket.logs.id`.
In CMake a `-` and a `.` are part of a target's name (`shop-core`, `shop.cli`), and `::` joins the
parts of an alias or an imported target, so `d` and `u` read `Shop::core` whole from either part;
a single `:`, as in `$<TARGET_FILE:shop_app>`, joins nothing. In Nix a `-` and a `'` are part of a
name (`my-package`, `x'`), a trailing `'` included, and a `.` separates the parts of an attribute
path.
Markdown reads a link or a code span whole around the cursor, not as a word.

## `D`: project symbols

`D` lists every declaration a single regex can recognise (`def class func function type fn struct
enum impl trait interface mod const static union macro_rules! namespace`, with
`export`/`pub`/`async`/`const`/`extern`/`declare` prefixes; `const` and `static` only unindented or
exported, since indented they are locals), plus shell functions (`name()`; the `function name` form
the single regex already finds), SQL `CREATE`d objects under the name as written (`public.orders`,
not CTEs), Makefile targets, Terraform blocks by address (`aws_s3_bucket.logs`, `data.T.N`,
`module.x`, `var.x`, `output.x`), Dockerfile stages and YAML anchors, each read only from its own
kind of file; GraphQL's `type`, `interface`, `input`, `enum`, `union`, `scalar`, `directive` (under
its name, without the `@`), `fragment` and named operations, from a row of its own, since the regex
above knows four of those words and would list them twice; a stylesheet's `@mixin`, `@function`,
`%placeholder` and `@keyframes`, under their names, and none of its selectors, custom properties
or variables, which Bootstrap alone has thousands of; recomputed on each press. Zig adds a function behind `inline` or `noinline` and a
`test`, under the description it is written with, which that regex has no word for. Java, Kotlin,
Scala, Ruby, C, C++, C#, Swift, PHP, Lua, Elixir, Protocol Buffers, PowerShell, Dart, CMake and Nix are read from rules of their
own instead of that regex — Java's types and its methods, told from a call by the return type before
the name; Kotlin's `fun` (past an extension's receiver), types, `object`, `typealias` and
`const val`; Scala's types, `trait`s, `object`s, `type`s and named `given`s, and its `def`s in a
row of their own; Ruby's methods, classes and modules, `def self.name` included; C and C++ functions,
methods, types, `typedef`s, `using` aliases and `#define`s; Objective-C's classes and protocols
(no category or extension) and its methods from the line of their definition, under the first part
of the selector; C#'s types, delegates and namespaces
behind their attributes and modifiers, and its methods and properties, told apart from a call the
way Java's are; Swift's types, `protocol`s, `actor`s, `typealias`es, `func`s and `extension`s, an
extension under the type it extends; PHP's types and `const`s in one row and its functions and
methods in another, behind `final public static` and the rest; Lua's functions in both of the
forms it writes them, under the name and not the table they hang off; Elixir's modules, protocols
and every `def` form; Protocol Buffers' `message`, `enum`, `service` and `rpc`, nested messages
included; PowerShell's `function` and `filter` under the whole `Verb-Noun` name, which that regex
would cut at its `-`, and its `class` and `enum`; Dart's types (`class` behind its modifiers,
`mixin`, a named `extension`, `extension type`, `enum`, `typedef`) in one row, its functions and
methods, told from a call by the type before the name, in a second and its getters and setters in a
third, which that regex would read as `class` for `abstract interface class Repo`; CMake's
`function` and `macro`, whose name stands inside the parentheses where that regex does not look,
and the targets of `add_library` (not an alias), `add_executable` and `add_custom_target` under
their names, no variable; Nix's bindings whose value is a function (`mkService = { name, … }:`,
`double = x: x * 2;`) with its set pattern on the binding's line (one wrapped onto the lines
below is not listed), and no other binding, since a NixOS module's options and a derivation's
attributes would flood the list — so none of them is listed twice or
under a modifier or a receiver. A C prototype is not listed, since every function of a header would
be there twice, and a `typedef struct x { … } y;` is listed once, under the `y` the project writes.
TypeScript's class methods, with neither a keyword nor a type in front, are not listed: the regex
cannot tell `name(` from a call. Neither are fields, a Protocol Buffers enum value, a C or Zig global, a Lua local, a C#
constructor (its class is already a row), a Swift `let`, `var`, `init` or `enum` case, a PHP
property, `enum` case, `define()` or magic method (`__construct`, `__toString`: the language's
hook, not the project's), a Ruby
constant, an Elixir module attribute or `defimpl`, or the names a Ruby `attr_accessor` or an
Elixir `defstruct` line declares, since one line can declare several. Markdown lists nothing: a
declaration in a README's code block is an example, not one of the project, and a heading is prose
that `s` finds.
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
A symbolic link to a directory is such a row too: the walk does not follow it, so a link to `..`
or `/` pulls nothing in, and a file behind it that lies outside the project opens read-only.

The open file itself is watched and reloads on every change on disk, keeping the cursor, the
scroll position, the jump history and the undo history, where the change is one more step.
