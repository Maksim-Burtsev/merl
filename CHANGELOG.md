# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `merl --drill [N]` trains the keys you do not press: N tasks on the tutor's sample project, 20
  by default, each saying what to do and never which key. A task counts when it is done with its
  own key, an alias included; done another way, or with `?` opened, it is a miss, and the same
  task comes again at once with the key named, then once more unnamed three tasks later. The
  status bar shows each answer's time, `✓ 0.8 s`. The keys unused or missed in the last 30 days
  of real work (`merl --keys`) and those missed or slow in the drill come most often, the keys
  never drilled first; every answer goes to `~/.local/state/merl/drill.tsv` right away. On exit
  merl prints the session's misses and slow answers. The drill counts nothing in `--keys`. (#209)
- `merl --keys` has a `missed` column: how often in the last 30 days a key would have done what
  you did another way in half the presses, saving three or more. `s` or `D` typed for the word
  under the cursor was `d` on its declaration, `u` on a use; `o` to a file one to three stops
  away in the jump history was `[` or `]`; a held or fast-tapped arrow, Shift+arrow, Backspace
  or Delete was the paging, word or line key that lands in the same place; in a review, a run
  into the next hunk, or `o` to the next file after the last one, was `c` / `C`. Only keys that
  work where you were count: `}` types in edit mode. Nothing shows while you work, and
  `--tutor` counts nothing. (#210)
- `merl --keys` prints how often each key has been pressed, in the last 30 days and in all,
  and when it was last: strongest first, so the keys you never press sit right above the prompt.
  merl counts them as you work, `--review` included, and adds the session's numbers to
  `~/.local/state/merl/keys.tsv` on exit; typing counts nothing, and neither does `--tutor`.
  An alias counts as its key: Ctrl+E as `o`. (#207)
- Review: a file `c` has walked to the end gets a tick in the panel, the last file on the `c`
  that has nowhere left to go. `m` puts the tick on the open file, or the panel's row, and takes
  it off. A ticked file that changes on disk loses the tick, on screen, while the agent works.
  The ticks last for the session; `c` / `C` still stop in ticked files. (#162)

### Changed

- `d` on a TypeScript name imported from a package installed more than once lands in the copy
  Node loads: the one in the nearest `node_modules` that has the package or its `@types`, with
  pnpm's link followed to the version it points at. A workspace root's copy, a copy another
  package depends on and another version in pnpm's store are no longer offered beside it, and a
  name only such a copy declares is found `by name`, no longer `via import`. A module of Node's
  own, such as `buffer`, is looked for as before, whatever npm polyfill of that name is
  installed. A workspace package linked into `node_modules` is the project's own: `d` finds it
  in its source, `by name`, and no longer in an old published copy another package depends
  on. (#141)
- `--tutor` sets every lesson up on its own: the sample project is unpacked anew, which drops
  what the last lesson edited, and the lesson opens on the file, line and word it is about. A
  lesson done the way it asks leads straight into the next; one that wandered off is put back.
  The lessons are the tasks the key drill will ask, one per key: the four that repeated a key
  (`t` to hide the tree, `w` to wrap again, `d` in a Makefile and on a declaration) are gone, the
  last two as a sentence in the `d` lesson, which leaves 21. The sample project has a long
  `notes.json` and a wide `export.csv` for the keys that page and wrap. (#208)
- Ignored files are in the tree, dim, as in VS Code's Explorer: a `.env` no longer needs
  `merl .env` from the shell. An ignored directory such as `node_modules/` or `target/` is one
  row, read from disk only when it is expanded, and followed live only while it is. `o` offers the ignored files of the walked
  directories (`.env`, `config/local.yml`), dim and after the rest; `s`, `u` and `d` search what
  they searched before. (#157)
- `c` / `C` outside a review do nothing and no longer say `not in review mode`. (#162)

- Opening another file no longer empties the undo history: each file keeps its own for the whole
  session, as a VS Code tab that stays open does. Fix a line, `d` to see what it calls, `[` back,
  and Ctrl+Z still takes the fix back; Ctrl+Y works the same. A file an agent wrote while you
  were in another one comes back with the write as one more step on top, as if it had been on
  screen. (#163)
- Search ignores case whatever letters the query has: `/`, `s`, `D` and `o` find `SameCancel`
  for `sameCancel`, where one capital used to make the whole query exact and find nothing. There
  is no switch; `u` still lists a word's uses in its exact case. (#174)

### Fixed

- Opening `/` again on a pattern that matches nothing in the file says `no match`, as typing it
  did, instead of `0/0`. (#174)
- `merl --review=feat` reads what the merge request shows. A local `feat` left from an earlier
  review was opened as it was, so a second review showed the old code, and the base stayed where
  this clone last fetched it, so what the branch took in from `main` looked like its own. Now the
  branch and the base are fetched together and the local branch is brought to what was pushed,
  after a force-push too. Your own commits are never rewritten: when the branch has diverged, or
  local changes are in the way, the status bar says `diverged from origin/feat` or `behind
  origin/feat`, and `origin/feat not fetched` when the fetch failed. (#181)

- `d` and `D` outside the project walk a Python directory once when `sys.path` lists it twice,
  apart (a `PYTHONPATH` entry, a `.pth` file): every hit there was offered twice. In a Rust
  project the cargo registry is listed once per `d`, not once per package of `Cargo.lock`. (#152)
- `merl FILE:LINE:COL`, what rustc, tsc, eslint, ruff and clang print, opens on that line and
  column instead of saying no such file; the column counts chars and stops at the end of the line.
  `FILE:LINE:` and a whole grep line quoted as one argument, `FILE:LINE:text`, open on the line,
  and a file really called `a:12` still opens as itself. (#161)
- A file that is one long line — a minified bundle, a one-line JSON dump, a source map — opens
  at once instead of freezing merl for seconds: a line longer than the 20 KB merl draws of it is
  no longer given to the highlighter and is shown plain, as VS Code stops colouring past
  `maxTokenizationLineLength`. The lines around it keep their colours. A 700 KB one-line
  `app.min.js` drew its first frame in 0.03 s instead of 4.1 s, and ten arrow keys over a `/`
  match took 0.04 s instead of 1.9 s; the `T` preview and the picker rows that quote such a line
  parsed it too, and no longer do. (#184)
- A row with an emoji of several code points (`⚠️`, `✔️` or `❤️` with their U+FE0F, a skin tone as
  in `👍🏽`, a joined `👨‍💻`, a keycap `1️⃣`) keeps all its text. merl measured the emoji a char at a
  time, one cell short or two cells long, so a full row lost its last letter at the right edge,
  and the cursor and the status bar column were a cell off, with typing landing beside the bar.
  The arrows, Backspace and Delete now take such an emoji as one step instead of stopping inside
  it and deleting half. (#185)
- `merl FILE` for a file in no git repository opens at once. It took the file's directory as the
  project and walked everything below it first, and for `~/.zshrc` that is the whole home: 15.6 s
  and 1 GB before the first frame, now 0.08 s and 14 MB. There the project is the files next to
  the one opened: the tree, `o`, `s`, `u` and `d` see them and nothing below. `merl DIR`, plain
  `merl` and a file inside a repository open the whole project as before. (#182)
- A file that starts with a UTF-8 byte order mark, as Visual Studio writes C# files and many
  Windows tools write scripts and exports, keeps the mark as its first bytes. It was the first
  character of line 1, so a line added at the top went in front of it, and the file stopped
  compiling (`invalid non-printable character U+FEFF`); Delete at 1:1 deleted the mark, and Right
  stepped over nothing. A file without the mark never gains one. (#177)
- Review: every line a branch deleted at the end of a file can be read. Down, PgDn and Ctrl+D on
  the last line scroll on through them until the last one is on the bottom row, as Up already
  brought in the deleted lines above a line; the cursor stays on the last line and Up takes the
  view back to it. Before, the view stopped at the last line and only the deleted lines that fit
  under it were ever on screen. (#179)
- `d` into the standard library and the dependencies no longer runs a program the project ships.
  In a Python project it ran `.venv/bin/python` to read `sys.path`, so a repository or a branch
  under review could run whatever it committed there on a keypress; the venv is now read, and the
  standard library is the one of the interpreter it was made from. Rust and Go are asked from
  outside the project: a `rust-toolchain.toml` no longer picks the `rustc` that runs, and a
  `go.mod` asking for a Go that is not installed no longer freezes `d` while Go tries to download
  it and then leaves `d` without the standard library for the session. A toolchain that fails to
  answer is asked again on the next `d`. (#183)

## [0.6.0] - 2026-09-21

### Added

- Ctrl+N makes a new file from the code, from edit mode or from the tree. The prompt asks for a
  path from the project root and starts in the directory of the open file or of the selected
  tree row; Enter creates the file with the directories it needs and opens it for editing, and
  the tree and `o` have it at once. A file that is there is opened, never overwritten; a path
  out of the project is refused. (#23)
- `d` in Go reads a build tag of the project's own as a plain `go build` does: unset. Of a type
  declared twice, under `//go:build gogit` and `//go:build !gogit`, `d` goes to the one that is
  built instead of asking; `cgo`, `gc` and `go1.N` count as set, `//go:build ignore` files drop
  out. `GOFLAGS=-tags=…` and `CGO_ENABLED=0` in the environment are honoured, and from inside
  the file under the tag both declarations are still offered. (#137)
- `d` no longer offers a line inside an embedded literal as a declaration: `literal_lines` reads
  a Swift or C# `"""` block, a C# verbatim `@"…"` (where `""` is a quote, not the end) and a PHP
  heredoc, so the SQL a migration embeds stops answering `d`. (#17)
- The project is live: a file created, deleted or renamed while merl runs (by an agent in the
  next pane, a `git checkout`, a build) shows up in the tree, in `o` and in what `s`, `u`, `d` and
  `D` search within a moment, with no key and no restart. The tree cursor stays on its entry and
  on its screen row, expanded directories stay expanded, an open picker keeps its rows until it is
  reopened. `.gitignore` is respected as at startup (a new `node_modules/` adds nothing) and an
  edited one is picked up. A burst of changes is one walk, off the UI thread; on Linux ignored
  directories are not watched (#75).
- The review is live: `merl --review` can stay open next to an agent working on the branch. A
  file it touches for the first time appears in the panel, the counts and `file 3/12` follow
  every save, commit, rebase and `git switch` within a moment, and a file whose changes were
  reverted leaves (the open one stays open, without marks). Untracked files that are not ignored
  are part of the review: listed as `A` with every line added, and `c` walks into them; a branch
  with nothing but a new, unadded module opens now. The open file, the cursor and both scrolls
  stay where they are, the panel cursor keeps its file, and nothing opens on its own. When lines
  are written above the hunk being read, the cursor goes down with its text, so the hunk stays
  the current one and `c` goes on from it. `git` runs off the UI thread; `HEAD` and the refs are
  watched explicitly, also in a linked worktree (#76).
- `d` and `D` in Zig, over every `.zig` file. `d` finds `fn name(` behind `pub`, `export`,
  `extern "c"`, `inline` and `noinline`, and the `const` or `var` the language declares everything
  else with — a type (`const Ledger = struct {`, `const Status = enum {`,
  `const Value = union(enum) {`), an import, a constant, a global and a local alike. A struct field
  has no rule, as a C field has none, and neither has a `test`: a word inside its description
  declares nothing, so `d` can never land on one. Zig has no literal that runs over lines — a
  `\\` string ends with its line — so the markdown a `\\` block holds is read as code. `d` leaves the project for the standard library
  where `zig env` says it is. `D` keeps the declaration pattern every language shares, which
  already reads Zig's `fn` and `const`, and adds the two forms it has no word for: a function
  behind `inline` or `noinline`, and a `test`, listed under its description. A `build.zig.zon` is
  painted as Zig but has no rules: the data format declares nothing. (#17)
- `d` and `D` in Elixir, over every `.ex` and `.exs` file. `d` finds every `def` form — `def`,
  `defp`, `defmacro`, `defmacrop`, `defguard`, `defguardp`, `defdelegate`, written with parens,
  with `do` or with `, do:`, a trailing `?` or `!` included — a `defmodule` or a `defprotocol`
  under the namespace it is written with, a `defstruct` field in either form, and a module
  attribute where it is given a value (`@timeout 5_000`), the `defstruct` line itself, not the
  continuation lines of a struct written over several. Several clauses of one function are
  several declarations and all are offered. `@spec`, `@type` and the other attributes the
  language and the libraries everyone uses own — ExUnit's `@tag`, Mix's `@shortdoc` — are
  directives, not declarations: `@spec parse(t) :: t` is a promise about `parse`, not its
  definition. A line inside an `@moduledoc """` heredoc declares nothing, as one
  inside a Python docstring does not. `D` lists modules, protocols and every `def` form from a
  rule of its own, where the pattern every language shares knew `def` and nothing else of the
  family and read the `x` of an anonymous `fn x -> …` as a declaration. (#17)
- `d` and `D` in Lua, over every `.lua` file. `d` finds a function in each form the language
  writes one — `function name(`, `local function name(`, `function M.name(`, `function M:name(`,
  `M.name = function(` and the `name = function(` of a table of handlers — and a `local`, one of
  several on the line included. A field holding anything but a function has no rule on purpose:
  `limit = 10` in a table constructor and a re-assignment inside a body are the same line, so `u`
  lists the uses instead. A `[[ ]]` or `[==[ ]==]` long string and a `--[[ ]]` block comment
  declare nothing, as a Python docstring does not. `D` lists functions from rules of its own, so
  `function M.setup(` is listed as `setup` where the pattern every language shares called it
  `M`, and `local function` is listed at all. (#17)
- `d` and `D` in PHP, over every `.php` and `.phtml` file. `d` finds a `function` (returned by
  reference too), a `class`, `interface`, `trait` and `enum`, a `const` and a `define('X', …)`, an
  `enum` case, a property with the type it carries and a constructor parameter promoted to one —
  all behind their `#[Attribute]`s and modifiers — plus an assignment that opens a line. A
  `case X:` of a `switch`, a `$key => $value` pair and `$this->name = …`, which writes to a
  property declared elsewhere, are not declarations. `d` leaves the project for Composer's
  `vendor/`, which is gitignored and so outside the project walk the way `node_modules` is, and a
  `use Illuminate\Support\Str` in column zero binds `Str` to that path, since PSR-4 spells a
  namespace the way the file system does. `D` lists the types and `const`s from one rule of its
  own and the functions and methods from another — two rows, because the hit cap is counted per
  row and one shared row would let a big project's methods crowd its classes off the list — so the
  declaration pattern every other language shares is untouched and nothing is listed twice; a
  property, an `enum` case, a `define()` and a magic method (`__construct`, `__toString`, the
  language's hook rather than the project's) are left out. (#17)
- `d` and `D` in Swift, over every `.swift` file. `d` finds a `class`, `struct`, `enum`,
  `protocol`, `actor`, `typealias`, `associatedtype` and an `extension` of a type — where a
  project keeps its own members of one, often the only place — a `func` past its generic
  parameters, `init`, `init?`, `subscript` and `deinit`, a `let` or a `var`, and an `enum` case,
  alone or among several on a line, with the associated or raw value it carries; all of them
  behind their `@attributes` and any modifiers, `private(set)` and a backticked name included. A
  `case .open:` or a `case let .open(x):` of a `switch` is a pattern, not a declaration, and a binding made by `if let` or `guard let` has no
  rule, since it rebinds a name declared elsewhere. `d` leaves the project for `.build/checkouts`,
  where SwiftPM keeps a package's dependencies as source; the standard library ships compiled,
  with no `.swift` file to read. `D` lists the types, the functions and the extensions from a rule
  of its own, so the declaration pattern every other language shares is untouched and nothing is
  listed twice; a `let`, a `var`, an `init` and an `enum` case are left out, as what a type holds
  is in every other kind. (#17)
- `d` and `D` in C#, over every `.cs` and `.csx` file. `d` finds a `class`, `struct`,
  `interface`, `enum`, `record`, `record class`, `record struct` and `delegate` past the generic
  parameters they declare and behind their `[Attribute]` lists and modifiers, a `namespace` under its last part, a
  `using x =` alias, a constructor behind at least one access modifier — a bare `Invoice(n)` is a
  call — and a method, a property, an event, a field or a local, told from a call by the type
  before the name, so `public int X { get; }`, `public string Name => _name;` and
  `int IComparable.CompareTo(o)` all count. An enum member has no rule: `Open,` in an `enum` body
  and in a collection initialiser are the same line, so `u` lists its uses. `d` stays inside the
  project, since a NuGet package ships compiled assemblies and the runtime's own source is not on
  the machine. `D` lists the types and the members from rows of its own, so the declaration
  pattern every other language shares is untouched and nothing is listed twice; a field and a
  constructor are left out, as in every other kind. (#17)
- `d` and `D` in C and C++, which are one kind over every `.c`, `.h`, `.cc`, `.cpp`, `.cxx`,
  `.hpp`, `.hh` and `.hxx` file, so a header finds what a `.c` or a `.cc` defines and the other
  way round. In column zero, where neither language has statements, `d` reads a function, a
  prototype, a signature that wraps and an out-of-line `Type::name(` definition; indented, it
  takes a method or a function only when its body opens on the line, so `return compute(x);` and
  `if (check(x)) {` are calls. It also finds `struct`, `class`, `union`, `enum`, `enum class`,
  `namespace` — behind a template head, a storage specifier and an attribute or export macro,
  a template specialization included — a `typedef` in every form, `using x =`, a `#define`
  (function-like too) and a global. When a header declares a function the project defines
  elsewhere, both are offered. An enum constant has no rule — `NAME,` in an `enum` body and in an
  initializer list are the same line — and neither has a field, a local or a template parameter:
  `u` lists their uses. `d` leaves the project
  for the system headers (the SDK `xcrun` reports, `/usr/include`, `/usr/local/include`,
  `/opt/homebrew/include`). `D` lists functions, methods, types, `typedef`s, `using` aliases and
  `#define`s from rules of their own, so the declaration pattern every other language shares is
  untouched and nothing is listed twice; a prototype and a `typedef struct x {` opening are left
  out, so a function and a type are one row each. A `.h` file now highlights as C++ instead of
  the Objective-C bat's syntax set gives it. (#17)
- `w` stops wrapping the open file: long lines are cut at the edge of the pane, `›` and `‹`
  mark a line with more to the right or left, and the view follows the cursor sideways (End shows
  the end of the line, Home the start). For Markdown tables, CSV and minified files, which
  wrapping takes apart. It is kept per file until merl quits, works in edit mode too, the status
  bar says `nowrap`, and `.csv` / `.tsv` files open that way. The tutorial has a lesson for it
  (#51).
- Shift+Left / Right extend the selection by a char, the step that was missing between a word
  and a line.
- `v` selects the word under the cursor, pressed again the line, then the paragraph between blank
  lines; a selection made by hand grows the same way. The tutorial has a lesson for it.
- Ctrl+C copies the selection in navigation too, with no trip through edit mode. Without a
  selection it still quits. Cmd+C / Cmd+X do the same from a terminal set up to pass them on
  (one Ghostty line, in the README), and Cmd+C never quits.
- Alt+Backspace (Option+Backspace on a Mac) deletes the word before the cursor and Alt+Delete the
  word after it, in edit mode and in every prompt and picker query; in edit mode each is one undo
  step.
- The `/`, `s` and `:` prompts and every picker query are edited in place, with the buffer's
  keys: arrows, Alt+arrows by word, Home / End, Delete, and Shift, Alt+Shift or Ctrl+Shift with an
  arrow, or Shift+Home / End, to select, so a typed-over or Backspaced selection replaces the
  query. Ctrl+A, Ctrl+E, Ctrl+W and Ctrl+U work as in a shell. `/` and pickers refresh on every
  edit, so `now` becomes `func now` without retyping it. Ctrl+letter no longer types the letter
  into a prompt.
- `/` reopens with the active query in the prompt, selected, as Cmd+F in a browser: typing
  replaces it, an arrow or Home edits it, so `now` becomes `func now` after a few `n` too. Esc in
  navigation clears the pattern and the next `/` opens empty.
- `d` follows a chain through a Python `@property`, `@cached_property` or
  `@functools.cached_property` and a TypeScript getter that declare their return type, so
  `self.repos.users.get_one` in a FastAPI service reaches the repository. One without a return
  type is where the chain breaks, and the search by name answers. (#87)
- `d` on `super().store` in Python and `super.store` in TypeScript lands on what the method
  overrides: the lookup of `self` / `this` started one level up, `store → Archive.store (via super
  of ColdArchive)`, where it used to list every `store` by name. Under several Python bases it
  jumps only when the answer needs no method resolution order. (#100)
- `d` reads a name from the innermost scope that declares it: a local hides a module-level or
  package-level name, a closure's variable the one of the function around it, a block's the
  function's, where the two used to disagree and leave a picker. A Python parameter named like a
  module-level `def` is the parameter. Two declarations in one scope that disagree, and an inner
  one with no readable type, are still a picker. On the name itself `d` lists the declarations of
  that scope only. (#100)
- A loop variable has the type of an element where the collection's type is written:
  `for repo in repos` with `repos: list[UserRepository]`, `for (const repo of repos)` with
  `UserRepository[]`, `for _, repo := range repos` with `[]*UserRepository` or `map[K]T`, written
  as an annotation, as the return type of the function the collection came from, or as Go's
  `make([]T, …)` and `[]T{…}`: `DeleteUser → UserRepository.DeleteUser (via repos:
  []*UserRepository)`. The collection itself, keys and pairs stay by name. (#100)
- `d` takes one more hop on a call: a local assigned from a method of a receiver whose type is
  proven has the return type that method declares (`info := e.RequestInfo()`,
  `repo = self.depot.people()`); a Python function with no annotation whose every `return`
  constructs the same class returns it, as TypeScript's already did; and a chain may hang off the
  call that starts it, `make_uow().users.delete_user`, `pkg.New(x).Run`, `new Depot().people`. A
  call of a call is still by name. (#100)
- A cast tells `d` the type: Python's `cast(T, x)` / `typing.cast`, TypeScript's `x as T`, Go's
  `v, ok := i.(T)` and the variable of `switch v := x.(type)` inside a `case T:`, assigned to a
  name or with the member hanging off the cast, `(x as T).find`, `i.(T).Find`. A cast to a type
  the project does not declare, a `case` of several types and `default` stay by name. (#100)
- `d` in TypeScript reads what prettier and the language write around a member (#100): a class
  header wrapped over several lines, a list of type parameters ending in `> extends Base<K> {`
  or the clauses over a lone `{`, no longer hides `this`, `super`, the fields and the bases of the
  whole class; a member access broken in front of its dots, `return this.db` over
  `.selectFrom(`, is one chain; `repo!.find()` and `uow?.users.find()` are the plain access;
  `const { repo, audit: trail } = this` hands the fields on; `new Local.Tool()` finds the class
  inside a namespace of the same file; a class a module declares under one name and exports under
  another, `export { Hono as HonoBase }`, is found by the import of the new name.
- `d` on a TypeScript `#private` member takes the name with its `#`, on the `#` and on the name:
  `this.#addRoute(` lands on `#addRoute(`, where it used to say nothing or find the public
  `addRoute`. (#100)
- In a workspace `d` looks for a TypeScript dependency in the `node_modules` of every directory
  from the open file up to the project root, the nearest first, where it used to read
  `<root>/node_modules` alone. Each is walked once; a package's own copy is listed by its path
  from the root. (#100)
- `d` proves more Go receivers: a package-level `var` declared in another file of the package,
  in a `var (` block or below the cursor (`defaultRepo.DeleteUser`); a type behind `type X = Y`,
  which is `Y`, where `type X Y` stays a type of its own; and a type declared once per platform
  (`clock_windows.go` beside a `//go:build !windows` file), where the file the host's `go build`
  compiles counts. A build tag that is no platform, two files that disagree and a question asked
  from inside a file the host does not build stay a picker. (#100)
- `d` on a Go package qualifier, `db` in `db.Get`, lands on the import line of the open file,
  `db: via import code.gitea.io/gitea/models/db`, where it used to list every `db` of the project
  by name. (#100)
- `d` proves more in Python. `from repos import UserRepository as Users` types a receiver as
  `UserRepository`. A module of the project that imports a name without declaring it, a package's
  `__init__.py`, hands it on: its own module-level imports are followed, `from .labels import *`
  included, four modules deep (`Session: via import store/sessions.py`); two sources, a name the
  module also assigns and an import inside a function stay by name. `Limits.MAX_USERS`, an `Enum`
  member and a dataclass field are read in the class body or a class above it
  (`RED → Color.RED (via Color)`); an attribute a method assigns to `self` is an instance's and is
  not. A class header wrapped over several lines keeps its name for `self` and its bases for
  `d` on a base's method. (#100)
- `d` on `Depot::open` in Rust, C++ and PHP answers `open → Depot::open (via Depot)` where it
  listed every `open` by name: the path in front of the word is joined with `::` as those
  languages qualify a name. Modules in front of the type count when the path starts inside the
  project (`crate::`, `self::`, `super::`, a file or a directory called so). The name of a type
  is no proof of which type: the project has to declare it once, and a `use` of the file must
  not bind the path's first name outside the project (`io::Error::new` behind `use std::io;`).
  `Self::`, a type that does not declare the word and a value's `.method()` stay by name. (#129)
- `d` says how it found the target. After a jump the status line reads
  `delete_user → UserRepository.delete_user (by name, 1 match)` or `load: via import json`. Over
  a picker the status line and the title read `delete_user: by name, 2 declarations`, and each row
  starts with the class, interface or receiver the declaration sits in and why it is listed. A
  module lookup that falls back past the module an import names, or a value whose name only
  matches a module, says `by name`. (#69)
- `d` on `x.word` where `x` is a value (a local, a parameter, `self.repo`) collects every method
  of that name, from the project and from the standard library and dependencies, instead of
  stopping: Python `def` in a class, TypeScript methods and signatures (read from `.d.ts` outside
  the project), Go `func (r *T) Name(`. One candidate jumps, several open the picker. (#69)
- `d` on a word an import brings in from the project's own code, or on a member of that module,
  searches only the file or package the import names, and says so:
  `UserRepo: via import app/repos.py`, `Open: via import store/`. Python absolute and relative
  modules and packages, TypeScript relative files, `index` files and `tsconfig.json` `paths`,
  default, named and namespace imports, Go packages under the project's `go.mod`. An alias finds
  the name it imports (`UR` behind `from .repos import UserRepo as UR`). A module that
  only re-exports the word falls back to the search by name. An import of anything else goes to
  the standard library and the dependencies before the project's same-named declarations, so
  `json.dumps` behind `import json` no longer lands on a project `dumps`. (#73)
- `d` on `x.word` or `x.f.word` in Python, TypeScript and Go reads the type of the receiver from
  its declaration and looks for the member in that type and in what it extends or embeds:
  `self.repo`, `this.repo`, a Go receiver's field, a parameter or a local, annotated, constructed,
  handed a parameter, or assigned from a call whose function declares its return type (one hop).
  The status line names the link:
  `delete_user → UserRepository.delete_user (via self.repo: UserRepository)`,
  `via NewRepo() *UserRepository`. Declarations in scope that disagree, such as a variable
  shadowed inside a nested function, keep the search by name. (#83)
- `d` follows a chain such as `self.uow.users.delete_user` one field at a time, up to six names:
  each field is looked for in the type before it, in what that type extends, and in the Go structs
  it embeds. The status line lists the links,
  `via self.uow: UnitOfWork → users: UserRepository`. A chain that cannot be followed falls back
  to the search by name and says where it broke: `delete_user: by name, 2 declarations (chain
  broke at users)`. (#85)
- `d` on the declaration of a member of an interface, a protocol, an abstract or a base class
  offers what implements it: `send: implementations of Notifier.send, 3 declarations`. The first
  `d` on a call still lands on the declaration the receiver's type names, so the implementations
  are two presses away, as they are with gopls. Python and TypeScript walk the types that name the
  declaring one, four levels deep; for a Python `Protocol` and a Go interface an implementation is
  a member of that name taking as many parameters. Only the project is searched. (#88)
- `d` lands on a field. On `issue.PosterID`, `self.repo` or `this.config` it said `no definition`
  although the receiver's type was proven; each type of the hierarchy is now asked for a method,
  else a field, through base classes and Go embedded structs:
  `PosterID → Issue.PosterID (via issue: Issue)`. By name, the project's fields join the method
  candidates, and on a field's declaration the other fields and members of the name are offered
  as its namesakes. (#104)
- Twenty-three themes, taking the set to forty-seven. The families Vim and Neovim users run most
  and merl was missing: gruvbox, One Dark, GitHub, VS Code's Dark+ and Light+, Dracula with its
  light Alucard, Nord, Solarized, Oxocarbon, Sonokai, Material and nightfox itself. Plus the
  variants of the families already here: `tokyonight-night` and `tokyonight-storm`,
  `catppuccin-macchiato` and `catppuccin-frappe`. Every one is ported from its Neovim original;
  `docs/themes.md` says which themes ship and why.
- Forty-seven niche themes from the Vim and Neovim world, taking the set to ninety-four: iceberg,
  gotham, jellybeans, PaperColor, vague, miasma, lackluster, mellow, alabaster, bamboo, edge,
  moonfly, nightfly, srcery, e-ink, mellifluous, ayu, vesper, adwaita, neomodern, darkearth,
  token, koda, soviet, cendre, selenized, pencil and minischeme — each with its light variant
  where it has one.
- Themes of your own: a `.tmTheme` in `~/.config/merl/themes/` is offered in `T` after the
  built-in themes and loads under its file name, and one named after a built-in replaces it, which
  is how a shipped theme gets copied and edited. A broken file is an error naming its path, never
  a silent fall back: at startup merl exits with code 1, in `T` it says so and keeps the theme on
  screen. (#58)

### Changed

- The pickers move by Up / Down only: Ctrl+P / Ctrl+N, which doubled the arrows, are gone, so
  Ctrl+N means one thing everywhere. (#23)
- `c` / `C` outside review mode say `not in review mode`, the fact without the `merl --review`
  hint, as the other status messages do.
- The README is rewritten around what merl is for: the one editor you need when agents write the
  code. The workflow, a new demo recorded on gitea and install come first, then the argument in a
  few sentences and "What merl is not"; the keys show the daily dozen with the rest folded. The
  editing and navigation reference moved to `docs/editing.md` and `docs/navigation.md`, a review
  walked step by step to `docs/a-day-with-merl.md`, the theme details to `docs/themes.md`, and the
  `Cargo.toml` description says the same as the tagline. The demo, the review demo and the
  walkthrough GIFs are recorded terminal sessions (asciinema and agg), one `.steps` file next to
  each, re-recorded with `assets/tapes/record.py`. The walkthrough only reviews: a fix typed over
  a branch under review reaches no pull request, so the Enter / Esc demo sits in
  `docs/editing.md` (#72).
- A reload from disk no longer empties the undo history: it is one step of it, as in VS Code and
  Vim. After an agent writes the open file, Ctrl+Z takes back what it wrote, line endings and the
  final newline included, and then the edits made before it; Ctrl+Y replays both. The step holds
  only the lines that changed. The edits Ctrl+R drops after a conflict are one Ctrl+Z away.
  (#122)
- `u` lists the same hits in the order a reader wants them: the declarations of the word first,
  each row marked `declaration`, then the open file, then the rest of the project's code with the
  nearest directories first, and tests, mocks, fixtures, generated and vendored files last
  (`tests/`, `__tests__/`, `spec/`, `testdata/`, `mocks/`, `vendor/`, `test_*`, `*_test.*`,
  `*.spec.*`, `*_pb2.py`, `*.gen.go` and friends). The title says how the list splits:
  `Usages of delete_user: 1 declaration, 6 in code, 14 in tests`. The candidates `d` offers are
  demoted by the same table, so a copy of a declaration under `spec/` comes after the real one.
  (#81)
- No silent keys: a press that cannot act says why, in a word or two. `d` and `u` off a word
  say `no word`; `d` in a file whose kind has no rules says `no rules for .css` instead of a
  `no definition` that never looked; `/` shows `no match` or the match count (`3/17`) next to
  the query while it is typed, and `n` / `N` keep the count, which replaces `wrapped`; Esc no
  longer says `find cleared` with nothing to clear; a file deleted on disk is named
  (`clock.c gone`). The status bar says `read-only` before Enter is pressed, and `--review`
  without a `git` binary names git instead of a bare OS error (#82).
- Ctrl+C copies everywhere and never quits: outside edit mode too it copies the selection, or
  the current line without one, and the status line says how much (`copied 3 lines`). In a
  prompt or a picker it does nothing. Quitting is `q`. Ctrl+X stays an edit-mode key. (#78)
- The word jump moved from Shift+Left / Right to Alt+Left / Right (Option on a Mac), where every
  other editor has it. Esc b / Esc f, which Ghostty, iTerm and Terminal.app send for Option+arrow,
  are the same jump and no longer leave edit mode, a prompt or a picker.
- `s` shows its hits while you type: the result picker opens at once with the query as its input
  line and refreshes after each pause, Up / Down move in it and Enter jumps. Enter pressed before
  the hits arrive waits for them. The title counts the hits, and shows `Search (…)` until the
  query on screen is answered, so `0 hits` always means nothing was found. The open file's hits
  come first even when a short query stops at 5000, and Enter on a query that found nothing says
  `no results for …` however soon it is pressed. Narrowing is done by typing more of the query;
  the fuzzy filter over the results is gone. (#53, #107)
- `D` searches past its cap: on a project with more than 5,000 declarations the list is only the
  ones found before the cut, so the query no longer filters those rows — it greps the declaration
  patterns for a name that matches it, after a pause in the typing, as `s` does. A name declared
  in a file the cut never reached is found that way. The title says which list is on screen:
  `Symbols (first 5232, type to search all)`, then `Symbols (…)` while the grep runs and
  `Symbols (94 hits)` for its answer, `5000+` when the cut caught that one too. Under the cap
  nothing changes. (#79)

- `via import database/sql` lists that package and no longer `database/sql/driver`: outside the
  project a Go import is one directory, the standard library's right under GOROOT's `src`, so
  `errors` is not `github.com/pkg/errors`. A package that is not installed is `by name`, not `via
  import` of the directory above it. (#100)
- A Go parameter or named result found as `local` reads `Load.err`, as a local of the body does,
  not `Issue.err`, which named a field. (#100)
- A count behind a grep that stopped at its cap says `+` also when a filter made the list short
  afterwards (`Pick: via import example.com/lib, 1+ declarations`), and the one candidate left
  is offered, not jumped to. (#100)
- A Python parameter found as `local` reads `RecipeController.get_one.slug`, as a local of the
  body does, not `RecipeController.slug`, which named a field. (#100)

### Fixed

- `d` reads the return type of a function whose signature holds `"\\"`: the scan the rules pair
  brackets and drop comments with took the escaped backslash for an escape of the closing quote,
  so the string ran on to the next quote in the file. `d` on `save` in `windows_repo().save(1)`,
  for `def windows_repo(sep="\\") -> Repo:`, offered every `save` by name instead of going to
  `Repo.save`. A Rust lifetime `'a` and a C++ digit separator `1'000` no longer open a quote in
  that scan either; nothing `d` does in Rust or C reads it yet. (#151)
- A picker over thousands of rows no longer holds up the keys typed after it opens: every row
  queued a redraw of its own, so on a 6,000-file project `o` took over a second to show the first
  letter of the filter. `u`, `d` and `D` read the open file first, so a list cut at 5000 hits
  keeps that file's hits. (#53)
- `d` no longer takes merl down on a word standing behind a character outside ASCII. Three places
  read the name in front of the cursor from the byte index `rfind` gives and added one to it,
  which lands inside a wider character: `d` on `load` in `данные.load(x)`, or on `word` in
  `café().word`, panicked out of raw mode and left the terminal unusable until `reset`. The name
  is read by characters now; one written outside ASCII is still no name to the rules, so `d`
  falls back to the search by name. (#150)
- Review: the lines a branch deleted can be read. They were drawn in the line-number colour with
  the terminal's `dim` on top, which in the default theme and many others left an empty-looking
  block beside the red bar, and a blank page for a deleted file. They are now the theme's text
  colour greyed toward the background, at 4:1 contrast or better (in the few themes whose own
  text is softer than that allows, 85 % of the text colour), the same in every terminal. (#144)
- `d` no longer proves a module-level namesake for a name a TypeScript destructuring wrapped
  over several lines binds (`const {` / `  ledger,` / `} = deps;`): the statement is read whole,
  and out of a name whose type is written the field's type is the local's. (#131)
- With the cursor on an interface method, a class that implements a namesake interface of
  another file through a barrel (`export * from`, `export { Name } from`) is no longer listed as
  an implementation, and neither is a class for the constraint of a type parameter on a line
  of its wrapped header, `S extends Notifier,`. A class that imports the interface as
  `type Notifier,` on a line of a wrapped list is listed again: the line read as an alias of
  that name. (#100)
- A Python binding that does not start its line is a binding: `if fresh: ledger = A()`,
  `else: …`, `a = 1; ledger = A()`, `try: from m import ledger`, `first = ledger = A()`, and
  `if cold: self.ledger = A()` for a field. `d` did not read them, so a module-level `ledger` of
  another type was proven in their place and jumped to. The same behind the last line of a header
  wrapped over several lines, `        cold): ledger = A()`. (#131)
- `d` on `x.member` no longer crashes merl in a Python file that continues a string with a
  backslash at the end of a line: the scan for docstrings lost count of the lines there. (#100)
- A Python import in a docstring's example is no second source of the name, which made
  `Depends` in fastapi's `applications.py` a picker of two modules. (#100)
- Under a Python class header wrapped over several lines, `self.get()` no longer skips the class's
  own `get` for a base's: the methods were named after nothing, so the class looked empty. (#100)
- A standard-library or dependency file stays read-only when it changes on disk or Ctrl+R
  reloads it; the reload made it editable.
- A Go `const` whose value spells a name no longer declares it: `const csp = "… http://…"` hid
  the `net/http` import of its file, so `http.Server` went by name into the project. Found by the
  hand pass of #100.
- SIGTERM, SIGHUP (a closed terminal or tmux pane) and SIGINT from outside end merl as `q` does:
  unsaved edits are written and the terminal is restored, instead of a shell left on the
  alternate screen with merl's last frame. (#80)
- The word jump and the word selection stop at words in any script: in a Russian comment
  Alt+Left / Right skipped the whole line, since only ASCII letters counted as a word.
- Review: `c` / `C` stop only where there is a hunk. Binary files, mode changes and pure renames
  are walked past, with `skipped 94 files without hunks` in the status bar, and the review opens
  on the first file that has a hunk; the panel still opens them with Enter. In the panel a binary
  file says `bin` instead of `+0 −0`, and a long name is cut with `…` instead of losing its
  counts. (#77)
- `d` finds a Python `async def` and a TypeScript method signature with no body
  (`find(id: string): User;` in an interface, an abstract class, an overload or a `.d.ts`). (#69)
- Ctrl+F opened while editing no longer drops you into navigation, where the next letter was a
  command (`d` jumped, `q` quit): Enter keeps editing at the match and Esc where you were. The
  same for Ctrl+G and for a prompt or a picker closed with Esc. (#56)
- Go lookups outside the project skip `_test.go` files, `testdata` and nested modules such as
  GOROOT's `cmd`, which no import reaches. A relative import (`from . import views`, `./utils`)
  and a Go package whose name its path decorates (`gopkg.in/yaml.v3`, `go-sqlite3`) now bind the
  name they bring in. (#69)
- A TypeScript `import { type Foo } from 'lib'` binds `Foo`, not `type`, so `d` on `Foo` looks in
  `lib`. (#73)
- `d` finds a TypeScript method whose empty body sits on its own line, `close(): void {}`. (#83)
- `d` on a member of a chain that hangs off a call, an index or `?.`, such as
  `make_uow().users.delete_user`, no longer reads a local `users` as the receiver and jumps into
  that variable's type: the word is a member of a value whose type is not known. (#85)
- A Python import no longer counts as a declaration of the modules on its path: with
  `from .guild import Guild` in the file, a parameter `guild` handed on to `self.guild` keeps its
  type, and `d` on `self.guild.x` reads it instead of falling back to the search by name. (#85)
- Eight themes had the selection colour within a few points of the cursor line highlight, so a
  selection inside the cursor line was nearly invisible: rose-pine-moon, rose-pine-dawn, nordfox,
  nightfox, gruvbox-material-light and material-light move the selection a step along their own
  palette, solarized-light darkens it, dracula lightens the line highlight instead. The theme test
  now requires the two colours to be apart.
- Review: `c` / `C` no longer stop dead in front of a submodule, with every file after it out of
  reach: a submodule is walked past like a binary file and stays in the panel. A file that does
  not open is walked past too, and the status bar says why it did not open. (#117)
- `[` / `]` no longer stop dead at a history stop whose file was deleted or renamed, with every
  stop behind it out of reach: the stop is dropped, the walk goes on to the next one that opens,
  and the status bar says `hist2.py gone`. (#118)
- The autosave no longer recreates a file that was deleted or renamed on disk, which left a stale
  copy next to the file an agent had just renamed. Gone is changed on disk: the conflict is raised
  as for a modified file, only Ctrl+S writes the file back, and Ctrl+R lets the edits go. (#119)
- An unbound Alt+letter (Option+letter on a Mac) in edit mode does nothing. It used to leave edit
  mode without a word, so the rest of the word ran as commands and its `q` quit merl. (#120)
- The `s` picker titles a single match `Search (1 hit)`, not `1 hits`. (#121)
- The descriptions in the `?` overlay, and the one-line hint a pane too small for the welcome
  keys falls back to, take the theme's readable grey instead of its line-number colour, which
  themes choose to disappear: under 3:1 against the background in 67 of the 94. (#146)
- A file merl cannot write is `read-only` from the moment it opens or is reloaded with Ctrl+R.
  Enter was accepted and the refusal came a second later with the autosave, after the keystrokes
  had nowhere to go. merl asks by opening the file for writing, not by the permission bits, which
  lie both ways: root writes a 444 file, an ACL or a read-only mount refuses a 644 one. (#123)
- The silent wrong jumps of `d` found by the #68 acceptance pass. A second `d` on a declaration
  offers its namesakes instead of jumping to one. A parameter or a local hides an import of the
  same name, so `def handler(json): json.loads()` no longer goes into the standard library, and
  a qualifier that is a value keeps the search to members. A name imported from outside the
  project is looked up at the top level of its module; one imported from two modules
  (`try:` / `except ImportError:`) offers both. A Go generic function, nested type parameters and
  a TypeScript constructor parameter property are declarations; a line inside a raw string, a
  docstring or a block comment is none. `Outer.find` is looked up in what `Outer` declares before
  any method called `find`. (#102)

## [0.5.0] - 2026-09-16

### Added

- `merl --review[=BRANCH] [--base REF]`: code review inside merl. The branch's diff against its
  base is a lens over the real files: added lines get a green mark, deleted lines are drawn in
  place as grey ghosts, and `d`, `u`, `s` work straight from the diff. The panel lists the
  branch's files with `M` / `A` / `D` and `+n −m`; `c` / `C` walk the hunks and cross into the
  next file; `[` comes back from an excursion. A deleted file opens read-only from the base.
  With `--review=BRANCH` merl fetches and switches to it; the base defaults to `origin/HEAD`, then
  `origin/master`, `origin/main`, `origin/develop`. (#60)
- Themes of your own: a `.tmTheme` in `~/.config/merl/themes/` is listed in `T` after the built-in
  ones and loads under its file name, and one named after a built-in replaces it, so a shipped
  theme can be copied and edited. A broken file names its path — at startup merl exits with code 1,
  and in `T` it says so and keeps the theme already on screen.

### Fixed

- Starting a selection inside a wrapped line no longer flashes its other rows back to the plain
  background: the cursor line keeps its highlight under a selection. Only a cursor line none of
  whose text is selected still drops it, so that line does not pass for selected (#48, #62).

## [0.4.0] - 2026-09-16

### Added

- Edit mode. Enter turns the cursor into a text cursor, Esc turns it back. Inside, merl is a plain
  editor with VS Code habits: letters insert, Enter splits the line and keeps its indentation, Tab
  indents the way the file already does, Backspace and Delete join lines, typing over a selection
  replaces it. There is no save step: edits reach the disk `autosave_delay_ms` (default 1000, a
  new config key) after the last keystroke and at once on Esc, on switching files and on quit;
  Ctrl+S saves now. A file changed on disk under unsaved edits is a conflict, not a reload: the
  status bar says so, Ctrl+S keeps the buffer and Ctrl+R takes the disk. Files keep their tabs,
  CRLF and trailing newline through a save; binary, non-UTF-8 and mixed-line-ending files stay
  read-only. Edits that could not be saved keep merl on the file: another file does not open over
  them and the first `q` is refused. The tutorial gets an editing lesson. (#11)
- Ctrl+Z / Ctrl+Y undo and redo. A run of keystrokes on one line is one step, as VS Code groups
  typing; a cursor move, a line split or join, or leaving edit mode starts a new one. The history
  is per open file. The tutorial gets an "Undo" lesson. (#11)
- In edit mode Ctrl+C and Ctrl+X copy and cut the selection, or the whole line without one, to
  the system clipboard through OSC 52, which the terminal forwards even over ssh. Paste is the
  terminal's own (bracketed paste), inserted while editing and ignored otherwise. (#11)
- Git marks in the gutter, as in VS Code: green for added lines, blue for changed ones, red under
  a line where lines were deleted, read from `git diff -U0` after every load, save and reload.
  (#11)
- `d` follows into the standard library and dependencies when the project has no definition:
  `sys.path` of the project's interpreter (a compiled module lands in its `.pyi` stub), the Rust
  sysroot and the crates in `Cargo.lock`, `GOROOT` and the modules of `go.mod`, `node_modules`.
  The file's imports say which module a qualified word (`json.load`, `fs::read`) comes from.
  Files outside the project open read-only. (#42)
- `d` and `D` in Rust: `fn`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`,
  `mod`, `macro_rules!` and `let` behind any `pub(..)` / `async` / `unsafe` / `extern` prefix;
  `impl` blocks are uses of the type, not definitions. (#25)
- `d` and `D` in TypeScript and JavaScript: `function`, `class`, `interface`, `type`, `enum`,
  `namespace`, `const` / `let` / `var` (so arrow functions assigned to a name), class and
  object-literal methods, behind any `export` / `default` / `declare` / `abstract` / `async`
  prefix. `.ts`, `.tsx`, `.js` and their module variants are searched together, so a `.tsx`
  component finds its types in `.ts`. (#31)
- `d` and `D` in Ruby. `d` finds `def`, `def self.name`, `class`, `module`, an assignment (a
  constant, an `@ivar`, a local), the names an `attr_accessor` / `attr_reader` / `attr_writer`
  line declares and an `alias` / `alias_method`, over every `.rb`, `.rake`, `.gemspec`,
  `.podspec`, `.rbi` and `.ru` file and `Rakefile`, `Gemfile`, `Vagrantfile` and friends; a
  trailing `?` or `!` is not part of the word, so `d` on `empty?` finds `def empty?`. `D` lists
  methods, classes and modules from a rule of its own, reading past the `self.` of a class method
  and the namespace of a `class Billing::Invoice`. Sorbet's `.rbi` files
  and `Dangerfile` now highlight as Ruby. (#17)
- `d` and `D` in Java and Kotlin, which are one kind: a `.kt` file finds the `.java` class it calls
  and the other way round, over every `.java`, `.kt` and `.kts` file. `d` finds Java's `class`,
  `interface`, `enum`, `record` and `@interface`, a method or constructor with a body, an abstract
  or interface method and a field, and Kotlin's `fun` (including an extension's receiver), `class`,
  `object`, `enum class`, `typealias` and `val` / `var`, behind annotations and the modifiers of
  either language. `D` lists them from rules of their own — Java's types and its methods, told from
  a call by the return type before the name, and Kotlin's `fun` (past an extension's receiver),
  types, `object`, `typealias` and `const val` — so the declaration pattern every other language
  shares is untouched and nothing is listed twice. (#17)
- `d` and `D` in SQL. `d` finds the `CREATE` of a table, view, index, function, procedure,
  trigger, type, schema, sequence, domain, extension, database, role or user — behind
  `OR REPLACE`, `TEMP`, `UNLOGGED`, `MATERIALIZED`, `UNIQUE` and `IF NOT EXISTS`, schema-qualified
  or quoted — and `WITH … AS (` common table expressions, across every `.sql`, `.psql`, `.pgsql`,
  `.mysql`, `.ddl` and `.dml` file; keywords ignore case. `D` lists the `CREATE`d objects under
  the name as written. `.psql`, `.pgsql` and `.mysql` now highlight as SQL. (#17)
- Highlighting for infrastructure files bat's set does not recognise by name: `.dockerignore` and
  `CODEOWNERS` (Git Ignore), `Containerfile` and `Dockerfile.*`, `*.jsonc`, systemd units and
  `.npmrc` (INI), `Procfile` and `yarn.lock` (YAML), `WORKSPACE` and `Tiltfile` (Starlark as
  Python). (#16)
- `d` and `D` in shell scripts. `d` finds `name()` and `function name`, an assignment behind
  `export` / `declare` / `local` / `readonly` / `typeset` (bare or `+=`) and an `alias`, across
  every `.sh`, `.bash`, `.zsh`, `.ksh` and shell dotfile (`.bashrc`, `.zshrc`, `.profile` and
  friends); `D` lists the functions, each once. (#17)
- `d` and `D` in Makefiles, Terraform, Dockerfiles and YAML. `d` finds Makefile targets and
  variables across every Makefile, the Terraform block behind `var.x` / `module.x` / `local.x` /
  `data.T.N` / `T.N` in the same directory, and Dockerfile stages, YAML anchors and block keys
  (compose services, CI jobs) in the same file. `D` lists targets, Terraform addresses, stages and
  anchors. A `-` is part of a word in these files. The tutorial gains a lesson on it. (#16)
- Twenty-one more themes, ported from their Neovim originals: Kanagawa Wave / Dragon / Lotus,
  Rosé Pine / Moon / Dawn, Everforest dark / light, Gruvbox Material dark / light, Catppuccin
  Mocha / Latte, Flexoki dark / light, Melange dark / light, Nordfox, Dawnfox, Dayfox, Tokyonight
  Day and Bluloco Light. `tools/port-theme.sh` ports a colorscheme in one command, and a test
  fails on a theme that paints code, YAML, Dockerfiles, Makefiles or TOML in too few colours. (#18)
- `T` picks a theme: the list repaints merl in the theme under the cursor as it moves, Enter keeps
  it and writes `theme` to `~/.config/merl/config.toml` (the rest of the file stays), Esc puts
  the old one back. Two new tutorial lessons teach it. (#18)

### Changed

- The file tree, `o` and project search include dotfiles (`.github/`, `.env`, `.dockerignore`);
  `.gitignore` still applies and `.git`, `.hg` and `.svn` are skipped. (#16)
- Dockerfiles use bat's `Dockerfile (with bash)` grammar: `RUN` lines are highlighted as shell and
  instruction arguments are no longer drawn in the default colour. (#16)
- All three themes colour the names infrastructure grammars emit: TOML keys and tables, INI
  sections, Terraform attributes, `.env` keys, Makefile and nginx variables, Dockerfile stages
  and image tags, YAML anchors and aliases. (#16)
- Long lines wrap between words instead of mid-word: a row ends after a space, and a word moves
  down whole. Only a word longer than the whole row (a URL, a call chain, a hash) is split where
  it stands, after `/ . , ; ) ] }` when it can. Rows after the first start under the text of the
  first, past the line's indentation and a list marker (`- `, `1. `), unless that takes more
  than half the pane.
- Up / Down, Shift+Up / Shift+Down, Ctrl+D / Ctrl+U and PgUp / PgDn move by screen rows, so a
  wrapped paragraph is walked row by row and half a screen is half of what the screen shows.
  Home / End, and Ctrl+Shift+Left / Right with them, stop at the start / end of the screen row
  first and go on to the line's on a second press, as in VS Code.

- A block cursor while navigating and a bar while typing and in every prompt, like vim, whatever
  the terminal's default shape is. (#41)
- `s>` project search looks for the text as typed, like `/`: `foo(` no longer fails as a bad
  pattern and `a.b` no longer matches `aXb`. There is no regex mode. (#52)
- `d` means definition: the whole-word fallback is gone, `u` lists uses. (#42)

### Fixed

- While text is selected the cursor line is highlighted in the gutter only, as in VS Code. A
  selection ending at the start of a line no longer looks like it takes that line, which Ctrl+C
  rightly leaves out. (#48)
- A `:` jump, a jump from a picker within the open file and a `/` search that moves the cursor
  drop the selection instead of stretching it to the new cursor; Esc on a search puts the
  selection back.
- A selection collapsed back onto its anchor selects nothing: Ctrl+C copies the line instead of
  an empty string, Backspace and Delete remove a char instead of marking the file changed, and a
  plain arrow moves.
- Ctrl+D / Ctrl+U and PgUp / PgDn only move the current history stop, so `[` after paging
  through a definition returns to the call site, not half a page back, and paging after `[` keeps
  the forward history. (#45)
- Quitting from the welcome screen or the `?` overlay left Ghostty and other xterm-like terminals
  without a cursor. (#46)
- Picking the open file in the `o` picker keeps the cursor instead of jumping to line 1. (#49)
- `d` finds a Kotlin `fun interface`. (#57)
- shokunin-light draws the selection in the theme's light blue instead of the cursor line colour.

## [0.3.0] - 2026-09-13

### Added

- `{` / `}` jump to the previous / next paragraph (blank line), like vim; in code that is the
  previous / next function without a parser. (#1)
- A welcome screen when merl starts without a file: the logo in the blues of the icon and the
  keys that work before a file is open. It shrinks to the keys alone, then to a one-line hint,
  when the pane is too small. (#9)
- `merl --tutor`: a vimtutor-style tutorial inside merl. Eighteen lessons over a sample Python
  project bundled in the binary, each one advancing when the key did what the lesson asked; the
  unpacked copy lives in a temporary directory and is removed on exit.

### Changed

- Find in file (`/`) matches literal text, as in VS Code: `migrator(` hits `Migrator()` instead
  of failing as an unclosed regex group. Smart case is unchanged; `s>` still takes a regex. (#5)
- A plain Left / Right on a selection collapses it to its start / end without moving further,
  as in VS Code; Up / Down still move from the cursor.
- The jump history now works like VS Code's: the current stop follows the cursor, so `[` goes
  back to where you were, not to where the last jump landed. A plain move farther than ten
  lines, Ctrl+D / Ctrl+U, `n` / `N` and find all add stops of their own; smaller moves update the
  current one. The history keeps the last fifty stops. Fixes #2.
- Enter in the tree on the file that is already open keeps the cursor where it is.

### Fixed

- A reload that shortened the file no longer crashes merl when a position taken before it is
  read back: Backspace to an empty `/` query, an arrow on a selection, `[` to an older stop.
  `[` onto a stop the file no longer reaches lands on the clamped line and keeps the forward
  history; a stop whose file is gone reports it and leaves the history alone.
- A line longer than 20 000 bytes (minified JS, single-line JSON) is wrapped the same way by
  the renderer and by the cursor arithmetic, so End no longer scrolls the line off the screen.
- Alt+letter over a picker, a prompt or the help only closes it; the letter is dropped instead
  of running a normal-mode binding (Alt+q used to quit).
- `u`, `d` and `s` search the open file even when the startup walk skipped it (hidden or
  ignored path opened by name).
- A result list cut at 5 000 hits says `(first 5000)` in the picker title.
- An invalid regex in `s>` is reported as `bad pattern`, not as `no results`.
- `?` scrolls with Up / Down when the terminal is too short for the whole list.
- The status bar says `no auto-reload` when no file watcher could be started, and `file gone`
  when the open file disappears from disk.
- Lines above a selection were painted with the selection background to the right edge.
- The usages, definitions and `s>` search pickers draw each hit with the syntax colours of the
  line it quotes, the same ones the code view shows after jumping there. Only the rows on
  screen are highlighted, so a picker over thousands of hits stays cheap. Fixes #7.
- A jump that went nowhere (`:` with the current line, for instance) erased the forward history.

## [0.2.0] - 2026-09-13

### Added

- Ctrl+D / Ctrl+U move half a screen, cursor and viewport together (vim/less style).
- A VS Code-style selection, painted with the theme's `selection` colour, that runs from where
  the first extending key was pressed to the cursor: Shift+Up / Shift+Down extend it by a line,
  Alt+Shift+Left / Right by a word, Ctrl+Shift+Left / Right to the start / end of the line (what
  Cmd+Shift+Left / Right does in VS Code, since Cmd never reaches a terminal program). Any other
  cursor move, Esc or opening a file clears it.

### Changed

- Shift+Up / Shift+Down no longer jump three lines; Ctrl+D / Ctrl+U cover fast movement.

### Fixed

- Go to definition finds annotated Python assignments (`NAME: Final[int] = ...`).
- Emptying the find query clears the previous highlights and returns to the anchor.

## [0.1.1] - 2026-09-12

### Fixed

- Symbol picker recognises `function` and declarations behind `export`, `pub`, `async` and similar prefixes (TypeScript, JavaScript, Rust `pub fn` were mostly missing).

## [0.1.0] - 2026-09-12

### Added

- Read-only file viewer with a line-number gutter and soft wrap at pane width.
- Syntax highlighting (syntect, bat syntax set) with tokyonight-moon, shokunin-light,
  shokunin-dark themes.
- CLI: `merl`, `merl DIR`, `merl FILE`, `merl FILE:LINE`, `--theme NAME`, `--version`.
- Project root resolution: the given directory, else the file's git toplevel, else its directory.
- Cursor movement: arrows, Shift+Up/Down (3 lines), Shift+Left/Right (word jump), PgUp/PgDn,
  Home/End, Ctrl+Home/Ctrl+End, with a sticky target column.
- Go to line via `:` or Ctrl+G; `q` and Ctrl+C quit, Esc closes the prompt.
- Status bar showing the path relative to the project root, the cursor position, and the focused pane.
- File tree in the left pane: `t` toggles it, Tab switches focus, Enter opens or expands,
  Left/Right collapse and expand; the tree reveals whatever file is opened.
- Fuzzy file picker (nucleo) on `o` / Ctrl+E, with matched characters highlighted.
- Find in file with smart-case regex: `/` or Ctrl+F searches as you type, `n` / `N` step through
  the matches with wraparound, and every match on screen is highlighted.
- Project search, go to definition (Python, Go), symbols and usages via ripgrep's library crates:
  `s`, `d` / F12, `D`, `u` / Shift+F12.
- Jump history: `[` and `]` walk back and forward through the positions a jump left behind.
- Config file `~/.config/merl/config.toml` with a `theme` key.
- Auto-reload: the open file is re-read when it changes on disk, keeping the cursor, the
  scroll position, the jump history and the find pattern.
- Help overlay on `?`, listing every binding; Esc in normal mode clears the find highlights.

[Unreleased]: https://github.com/Maksim-Burtsev/merl/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.6.0
[0.5.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.5.0
[0.4.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.4.0
[0.3.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.3.0
[0.2.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.2.0
[0.1.1]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.1.1
[0.1.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.1.0
