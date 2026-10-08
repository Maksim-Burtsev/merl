# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `f` folds Objective-C, `.m` files and the `.h` headers that hold Objective-C, where it said
  `no fold rules for .m`: on top of C's folds, an `@interface`, `@implementation` or `@protocol`
  to its `@end`, a method, a method declaration or `@property` wrapped over lines, a block, an
  `@[…]` or `@{…}` literal, `@try` with its `@catch` and `@finally`, and `@autoreleasepool`.
  (#625)
- Pictures, where merl draws Mermaid (Ghostty, kitty, WezTerm): a PNG, JPEG, GIF, WebP, BMP, ICO
  or TIFF file opens as the picture, centred in the code pane, shrunk until it fits and never
  enlarged, its transparent parts over a grey checkerboard, and its size, `1185×960`, in the
  status bar where the line and column stand. A GIF or an animated WebP plays in a loop. `p` on
  an SVG draws it, `p` again shows its source. The Markdown preview draws a local image that
  stands on its own line, `![alt](path)` or an `<img>` tag with its `width` and
  `align="center"`, taking the theme's variant of a `<picture>` or of `#gh-dark-mode-only`;
  images from the web and inside a sentence stay `▣ alt`. A file merl cannot read whole stays
  as before and the status bar says why (`AVIF not drawn`); a terminal that draws no pictures
  names it in the note, `PNG 1185×960, not shown`. (#760)

### Changed

- `--review`: a file renamed without changes shows, where its text would be, the old and the
  new path with the part that changed on the review's word tints
  (`app/util/text.py  →  lib/text.py`), `renamed, no changes` and `c  next file`, as GitHub and
  GitLab hide its text; the line breaks in two when the pane is too narrow. Keys that act on the
  text do nothing there; `c`, `C`, `o`, `s`, the panel, `?` and `q` work as anywhere. (#746)

### Fixed

- `f` in Swift no longer folds the parameters of a function named in backticks, ``func
  `default`(``; in PHP a `#` comment right after a word, `$y# note }`, no longer ends the block
  at its `}`, and an anonymous class, `new readonly class extends Base {`, no longer folds as a
  declared one. (#625)
- `f` no longer crashes merl in a C or C++ file whose last word is `do` or `try`, as a file left
  half-written ends. (#756)

## [0.8.3] - 2026-10-07

### Added

- The tree marks a file changed since the last commit with `M` and a new one with `A`, staged or
  not, in the review panel's column before the name, and follows the disk as an agent writes
  (#402).
- `p` draws a Markdown file's Mermaid diagrams as pictures, in the theme's colours, in place of
  their source, in terminals that draw pictures (Ghostty, kitty, WezTerm; not inside tmux). A
  diagram wider than the pane shrinks to it, down to half its size; a wider one, one the renderer
  cannot read, and every diagram in other terminals stay their source. Building merl needs Rust
  1.95. (#727)
- `d` on a C++ member reads the class its receiver is declared as: `key.size()` with a
  `const Slice& key` lands on `Slice::size` (`via key: Slice`) instead of offering every `size`
  of the project. The receiver is a parameter, a local (`auto x = new T(…)` included), a field
  of the class the method is in, or `this`; `std::unique_ptr<T>` is `T`, a member the class
  inherits is found in its bases, and a chain names each link
  (`via thread: ThreadState → shared: SharedState`). A receiver of a `std::` type never lands in
  the project: `v->clear()` on a `std::vector` searches the headers outside it. One whose `->`
  reaches a type of the project, `std::optional<Tariff>` or `std::vector<Tariff>::iterator`, and
  any qualifier the rules cannot place (a namespace alias, a namespace a macro opens) are searched
  by name as before.
  `it->Valid()` on an `Iterator* it` lands on the declaration in `class Iterator`, where it
  offered the overrides beside it; a receiver whose type is not known still offers them (#373). (#389)
- `d` in four more places of stylesheets. `styles.container` behind
  `import styles from './Button.module.css'`, or a name a named import takes from a CSS Module,
  lands on `.container {` in that file only, `container: via import src/Button.module.css`, where
  it said `no definition for container`. A Sass variable, mixin or function the project does not
  declare is found in `node_modules` from a Sass file, a Less variable from a Less file, and a
  class of a `.sass` file is found in its indented rules, `&__item` and all. (#590)

### Changed

- A tree too narrow for its names widens while it has the keys: on Tab, when a row on its screen
  is cut at 30 columns, the tree widens to the longest row on its screen, at most half the
  screen, and keeps that width until Tab back to the code returns it to 30 columns; a longer
  name found during the visit widens it on the next Tab. Where every name fits, nothing moves.
  A chain of directories that each hold only the next one, too deep for the names below it to
  keep 12 columns, shares one row from its lowest directories, cut from the left at a `/`
  (`…/twofactor/totp`); Left, Right and Enter work on the shared row. (#399)
- Line numbers and the `‹` `›` `…` marks read at 3:1 on the background in every theme: a theme
  whose own gutter colour is fainter has it lightened (darkened in a light theme) in its own hue
  until it gets there, never past the theme's comments. 50 of the 94 themes change, the default
  among them (1.3:1 before); the rest draw as before. (#555)
- `d` outside the project in JavaScript and TypeScript lists a declaration once when another
  package keeps a copy of its own: a line of the copy of `typescript` another package keeps is no
  longer listed beside the same line of the copy Node loads from the open file. A member is
  looked for in the packages the file imports before every installed one, so its picker lists
  those alone. (#318)
- `c` and `C` in `--review` stand on the first line a hunk added, where they stood on the first
  line it deleted: on a hunk that rewrites code, the cursor is on the new code, the old code
  above it, a key Up away. A hunk that only deletes still stands on its first deleted line.
  (#690)
- `v` grows on to the whole file: a fourth press, after the word, the line and the paragraph,
  selects every line of the file, the cursor at its end; Ctrl+C then copies it all. (#635)
- Shift+PgUp / Shift+PgDn, Shift+Home / Shift+End and Ctrl+Shift+Home / Ctrl+Shift+End extend
  the selection over everything the cursor passes, as Shift with an arrow does, where they moved
  the cursor and dropped the selection; the same moves without Shift drop it, as in VS Code.
  (#687)
- The lists of `u`, `s` and `d`'s choices name a file once, above its rows, and each row shows
  the line number and the code: `polar/license_key/endpoints.py` over `242  license_key = await …`
  instead of the path repeated on every row. A row still too wide for the list wraps under its
  code instead of stopping at the border mid-word. `u` no longer writes `declaration` before
  its first rows: the title counts them. With the tree open and the code at least 80 columns
  wide, every list sits over the code instead of over the tree, its text where the code starts. (#479)
- The first `d` that leaves the project no longer waits for the walk of the standard library and
  the dependencies (about 0.9 s for Rust): merl walks them in the background once a file of that
  language opens. Rust's standard library is read without its tests and benches. (#318)
- A `d` in JavaScript or TypeScript that searches the dependencies by name no longer reads all of
  `node_modules` again on every press: the lines of each file that could declare something are
  kept after the first. On eslint, 9 in 10 presses take under 160 ms, where they took up to
  400 ms. (#318)
- `d` in TypeScript and JavaScript reads the type of a receiver in five more places. In
  `function Row({ apiKey }: Props)` `apiKey.id` lands on the `id` its `Props` field's type
  declares, as `auth.user` does after `const { auth } = useStores()`, `membership.permission`
  inside `for (const membership of document.memberships)`, and `server.post` after `const server
  = getTestServer()` whose body does `const server = new TestServer(); return server;`. Each
  offered a picker of every namesake. `props.href` with `props: Omit<Props, "document"> & { href:
  string }` lands on the `href` of the literal, where it jumped to another class's `href`, and `d`
  on `rest` after `const { id, ...rest } = document` lands on that line, where it said "no
  definition". (#354)
- `--review`: `c` and `C` stop on every file of the review, an empty one included (a new
  `__init__.py`, a pure rename, a mode change), and the review opens on one when it comes first,
  a deleted one too; `c` ticks it when it leaves it, as any other file. An empty file's stop
  says `empty file, added` (or `deleted`, `renamed from …`) and `c  next file` where its text
  would be. Only binary files and submodules, which merl cannot show, are still walked past.
  (#715, #720)
- `/` and `s` open with the selected text as the query when the selection is within one line,
  already searched and selected so typing replaces it; without a selection, or with one over
  several lines, they open as before. (#410)
- Undo steps close as in VS Code: a cursor move ends the step, so typing after the cursor went
  away and came back is undone on its own; so does a switch between typing and deleting, and a
  space typed after a word starts one. A cut, a paste and Tab are steps of their own; Enter starts
  a step that the typing after it joins. (#478)
- The selected row of every picker (`o`, `s`, `u`, `D`, `d`'s choices, `T`) and the tree's cursor
  while the tree has the keys take the theme's selection colour as it is, as LazyVim's picker
  and tree do, instead of the cursor line's, which could barely be seen (1.09:1 in the default
  theme). Eight themes take their selection back from their Neovim original: tokyonight-moon
  selects in its blue, rose-pine-moon, rose-pine-dawn, nightfox, nordfox, gruvbox-material-light,
  solarized-light and material-light in their own colour. Selected code is no longer repainted
  grey over a review's added and deleted rows. (#556)

### Fixed

- `--review` opens on its first file when the branch deleted it, where it opened on the second
  and only `C` reached the first. (#720)
- `d` on a Java or Groovy call lands on the right overload when a parameter's annotation or
  default value holds a comparison: the `<` of `@Max(LOW < HIGH ? 1 : 2)` or of
  `boolean fast = pace<1` no longer hides the parameters after it. (#700)
- `d` in PHP on a relative namespace segment in a file of several `namespace` blocks, `Legacy` in
  `Legacy\Entry::OPEN`, reads it in the block it stands in. It read it in the file's first block
  and said `no definition`. (#617)
- `d` in a Vue or Svelte component on a member of a `v-for` or `{#each}` item named like a
  `const` of the script above, `theme` in `{{ settings.theme }}` under `v-for="settings in
  rows"`, no longer jumps to the key of the script's `const settings = { theme }`: the item is
  another value, and `d` searches the member by name. (#618)
- Ctrl+Shift+D no longer opens the symbols list, as `D` does, in Ghostty, kitty and WezTerm:
  a Ctrl chord with Shift is never read as the bare letter, and one merl does not bind does
  nothing. (#688)
- In `--review`, a file the branch deleted draws the red `▎` of a deleted line on every line,
  where it drew the `▁` that marks lines deleted below one. (#642)
- `d` on `Shuttle` in a Groovy `new Shuttle(m)` lands on `Shuttle(@Named("cfg") Map<K, V> m)`
  instead of offering a picker with `Shuttle(String s, int n)`: an annotation with arguments no
  longer hides the commas of a generic type, or a default value, behind it. (#679)
- `d` in Clojure on the alias before `/`, `version` in `(version/in-range? v version)`, lands on
  `in-range?` in the namespace the `ns` form requires under that name, or on the namespace's file
  when it does not define it. It jumped to a local `version` in scope, or said `no definition`.
  (#735)
- `d` in Common Lisp on a variable `loop` binds, `line` of `(loop for line in data …)`, lands
  on its `for`, `as` or `with` clause, a destructuring `for (key value) in …` included, where it
  jumped to a global of the same name or said `no definition`. (#734)
- `d` in Emacs Lisp on a variable bound earlier in the same `when-let*`, `if-let*`, `and-let*`,
  `pcase-let*` or `let*` list, `limit` in `(when-let* ((limit limit) (limit (* 2 limit))) …)`,
  lands on the binding just above it, where it jumped to an outer `let` of the same name. A
  binding's own value still reads the one outside it. (#731)
- `d` in Racket on a variable of a named `let`, `(let loop ([xs xs] [n 0]) …)`, or on a name an
  `inherit`, `inherit-field`, `init-field` or `field` clause of the class around binds, lands on
  that binding, where it jumped to a namesake elsewhere in the project. (#733)
- `d` in an R7RS project (chibi-scheme) on a parameter of an `opt-lambda` lands on it; a
  definition in a `cond-expand` branch other than the one that includes the file is no longer
  jumped to; and a `.scm` or `.sld` file no longer searches Racket's collections, where `d`
  offered their files. (#732)
- Selected text takes the colour its theme's Neovim original gives it: srcery and lackluster
  painted it in the selection's own colour, so it could not be read; solarized, material,
  hojicha, neomodern-light, miasma and papercolor change their selected text too. (#686)

## [0.8.2] - 2026-10-04

### Added

- `f` folds Go, JavaScript, TypeScript (JSX and TSX included), Rust, C, C++, C#, Java, Kotlin,
  Swift and PHP, where it said `no fold rules for .go`: on a line that opens a construct it folds
  what Neovim's treesitter folds there in that language (a function, a class, an `if` and its
  branches, a `switch`, a loop, an object, an array, and where the language's folds have them, a
  `case`, a call's arguments wrapped over lines, a JSX element, a C `#ifdef` to its `#endif`, a
  Rust `impl`, a run of imports, a Go composite literal and each of its elements); in C, C++,
  C#, Java and Rust, on the header line of a function, class or `if` whose `{` stands on the
  next line it folds that body, and `f` there again unfolds it; anywhere else inside a function
  it folds the function; a raw string or a template literal at column 0 inside a body does not
  end it, and the HTML around PHP's `<?php … ?>` is not read as code. An Objective-C header still
  says `no fold rules for .h`. (#625)
- `f` folds Ruby, Lua and shell, where it said `no fold rules for .rb`. On a line a word opens
  (`def`, `class`, `module`, `if`, `unless`, `case`, `while`, `for`, `begin`, `do`, a lambda;
  Lua's `function`, `if`, `for`, `while`, `repeat`, `do`, a table or a call wrapped over lines;
  shell's functions, `if`, `case`, `for`, `while`, `until` and heredocs) it folds that block, its
  `end`, `fi`, `done` or `esac` shown after the `⋯`; on `else`, `elsif`, `when`, `rescue`,
  `ensure`, `elseif` or `elif` it folds that branch; anywhere else inside a method or function it
  folds the method or function. (#626)
- `f` folds YAML, JSON, TOML, HTML, CSS / SCSS and Markdown too, where it said
  `no fold rules for .json`: a key with what is nested under it and a list item in YAML, an
  object or an array in JSON, a table and a wrapped array in TOML, an element from its start tag
  to its end tag in HTML, a rule in CSS from the first line of its selectors, and in Markdown a
  heading with its section, a list and a code block. On a line inside one, `f` folds the
  innermost one around it. (#627)
- `d`, `u` and `D` in Groovy, Gradle build scripts and Jenkinsfiles, where `d` said `no rules for
  .groovy`, `no rules for .gradle` and `no rules for this file`. `.groovy`, `.gvy`, `.gradle`,
  `*.jenkinsfile` and `Jenkinsfile` are one kind with Java, Kotlin and Scala, so a Groovy class
  finds the Java class it calls and back. `d` on `buildDocs` in `dependsOn 'buildDocs'` lands on
  its `tasks.register('buildDocs', Copy)` or `task buildDocs {`; on `kotlinVersion` in
  `"${kotlinVersion}"`, on `ext.kotlinVersion = …` or its line inside `ext {`; on `buildPlugin`
  in a Jenkinsfile, on `def call(` of the shared library's `vars/buildPlugin.groovy`. It finds
  `trait`, a method or a variable with `def` and a method's parameters; a Gradle or Jenkins block
  or call (`dependencies {`, `node {`, `sh 'make'`) declares nothing, nor does a line inside a
  `'''` or `$/ … /$` string. `D` lists Groovy's `def` methods and Gradle's tasks, and no `def`
  variable. (#423)
- `d`, `u` and `D` in Erlang (`.erl`, `.hrl`, `.escript`), searched with Elixir as one kind,
  where `d` said `no rules for .erl`. `d` on `total` in `shop_order:total(X)`, or in Elixir's
  `:shop_order.total(x)`, lands on its clauses in `shop_order.erl`, `total: via shop_order`; on
  `#order{` on the `-record(order, …)`, on `total` in `R#order.total` or `#order{total = T}` on
  that field, on `?DEFAULT_TOTAL` on its `-define`, and on `-include("shop.hrl")` it opens the
  header. A local call lands on the clauses of its own module or of the one its `-import` names. Mix's `deps/`, rebar3's
  `_build/default/lib` and the sources of the installed OTP are searched too. `D` lists modules,
  records, macros, types and function clauses. (#425)
- `d`, `u` and `D` in Haskell, where `d` said `no rules for .hs`. `d` on `formatPrice` behind
  `import Shop.Money (Money (..), formatPrice)` lands on its signature in `src/Shop/Money.hs`,
  `formatPrice: via import src/Shop/Money.hs`; on a `go` a `where` binds, on that `go` in the same
  file, `go: local`, never another module's; on `lookup` in `Map.lookup` behind
  `import qualified Data.Map as Map`, `no definition for lookup`, never the project's own `lookup`.
  `D` lists signatures, functions with none, `data`, `newtype`, `type`, `class` and `pattern`, and
  `u` reads `foldl'` as one name. (#426)
- `d`, `u` and `D` in OCaml (`.ml`, `.mli`) and F# (`.fs`, `.fsi`, `.fsx`), where `d` said
  `no rules for .ml`. `d` on `format_price` in `Money.format_price` lands on its `let` in
  `lib/money.ml`, `format_price: via Money`, never on the `.mli`'s `val`; from an `.mli`, on the
  `.ml`; on a name a local `let` binds, on that `let` in the same file, `prefix: local`, never on
  another file's. It finds `let`, `type` with its constructors and fields, `module`, `val`,
  `exception`, `external`, `class` and `method`, and F#'s `member`, `override`, `abstract` and
  `namespace`. `D` lists each module's items, and `u` reads `x'` as one name. `List.map` is
  looked for in the OCaml standard library and the opam switch. (#427)
- `d`, `u` and `D` in Julia, where `d` said `no rules for .jl`. `d` on `format_price` lands on
  its one-line `format_price(m::Money) = …`, on `check` in `@check` on its `macro check(ex)`, on
  a name the function assigns or takes on that line, `m → report.m (local)`, never on another file's global
  `m`; on the path of `include("../src/money.jl")`, the file; on `DataFrame` or a name
  `import DataFrames: select` binds, the installed DataFrames at the version `Manifest.toml`
  pins, read-only, and on Base and the standard library in the Julia on the PATH. `D` lists
  functions, one-line methods, structs, abstract and primitive types, macros and modules, and
  `sort!` is a name of its own for `d` and `u`. (#429)
- `d`, `u` and `D` in R (`.R`, `.r`, `.Rprofile`), where `d` said `no rules for .R`. `d` on
  `format_price` lands on its `format_price <- function(cents) {`; on `label` in `cart$label()`, on
  the R6 method `label = function()`; on `area`, a picker of its `setGeneric` and every
  `setMethod`; on the path in `source("R/money.R")`, that file. `filter` in `dplyr::filter(x)` says
  `no definition for filter` rather than landing on the project's own `filter`. `D` lists functions,
  S4 and reference classes, generics and R6 classes, and `u` reads `print.invoice` and `.onLoad` as
  one name. (#422)
- `d` and `D` in Perl (`.pl`, `.pm`, `.t`), where `d` said `no rules for .pm`. `d` on `new` in
  `Shop::Order->new` lands on `sub new` of the file that declares `package Shop::Order`, `new: via
  Shop::Order`, and on `Order` there on that `package` line; on `$order->total`, on `sub total` by
  name; on a `my` or `state` variable, on its nearest declaration in the same file, `self: local`,
  never on another file's `my $self`; on `basename` behind `use File::Basename qw(basename)`, on
  its `sub basename` in `@INC`, read-only. A constant, a `has` attribute, an `our` variable and
  5.38's `class`, `method` and `field` are found too, nothing in POD, a heredoc or after
  `__END__`, and `D` lists the subs, methods, packages and classes. (#424)
- `d`, `u` and `D` in GDScript, Godot's language, where `d` said `no rules for .gd`. `d` on a
  `func`, a `class_name`, an inner `class`, a `signal` (from `died.emit()` or
  `emit_signal("died")`), a member `var` or `const` or an `enum` value lands on its declaration;
  on a `var`, a `for` variable or a parameter of the function around the cursor, on that line,
  `direction (local)`, never on another script's `direction`. `GameState.add_coins` lands in the
  script `project.godot` autoloads as `GameState`, `add_coins: via GameState`, and a
  `res://` path opens its file. A node path (`$Sprite2D`, `%HealthBar`) names nothing. `D` lists
  functions, classes, enums, `class_name`s and signals. (#434)
- `d`, `u` and `D` in Solidity, where `d` said `no rules for .sol`. `d` on `withinSupply` in a
  function's header lands on its `modifier withinSupply(uint256 amount) {`, on `Minted` in
  `emit Minted(…)` on its `event`, on `MAX_SUPPLY` on its `uint256 public constant MAX_SUPPLY`; on
  a name an `import {ERC20} from "@openzeppelin/…";` binds, on its `abstract contract ERC20` in
  `node_modules`, `ERC20: via import @openzeppelin/contracts/token/ERC20/ERC20.sol`, a path
  resolved through `remappings.txt`, `foundry.toml` and `lib/` too; on the import's path, that
  file; on a parameter or a local, the line that declares it. `D` lists contracts, libraries,
  modifiers, events and errors beside functions, structs, enums and interfaces. (#433)
- `d` follows a `$ref` in OpenAPI and JSON Schema files (`.yaml`, `.yml`, `.json`). On `User` in
  `$ref: '#/components/schemas/User'` it lands on `components/schemas/User`, where it offered
  every key named `User`; on `'./schemas/order.yaml#/Order'` on `Order:` in that file, where it
  said `no definition for Order`; in a `.json` file on `"#/$defs/address"`, where it said
  `no rules for .json`. An escaped key (`~1users`), an item index, an `$anchor`, a URL a
  project file declares as its `$id`, and a `discriminator.mapping` value are followed too; a
  `$ref` in a block scalar or a comment is not. (#435)
- `d`, `u` and `D` in Clojure, Emacs Lisp, Scheme and Racket, and Common Lisp, where `d` said
  `no rules for .clj`, `.el`, `.scm`, `.rkt` or `.lisp`. `d` on `format-price` in
  `money/format-price` lands on its `(defn format-price` in the file of the namespace the `ns`
  form requires as `money`, `format-price: via import src/shop/money.clj`, and finds nothing for
  `str/join` of a namespace outside the project; on a name a `let` or the parameters of a `defn`,
  `defun`, `define` or `lambda` bind, on that binding, `total: local`; on `(require 'shop-money)`
  or `(require "utils.rkt")`, the file. `D` lists each dialect's functions, macros and types, and
  `u` reads `format-price`, `empty?` and `*out*` as one name. (#428)
- `d` and `D` in Starlark: Bazel's `BUILD`, `*.bazel` and `.bzl` files, `.star`, Tilt's `Tiltfile`
  and Buck's `BUCK`, where `d` said `no rules for .bzl`. `d` on a macro or a rule a `load` takes
  lands on its declaration in the file the `load` names, `shop_binary: via import tools/defs.bzl`;
  anywhere on a label, on what it names: `":api"` on the target's `name = "api"` line, `"api.go"`
  and `"//tools:defs.bzl"` on the file, `"@rules_go"` on its `bazel_dep` line, and in a project
  Bazel has built, `"@rules_go//go:def.bzl"` and the names loaded from it in the repository Bazel
  fetched, read-only. `D` lists the rules, the providers and the targets. (#431)
- `d` in Kotlin follows the names Gradle and Android generate from files that are no code, where
  it said `no definition`: in a `build.gradle.kts`, `d` on any segment of
  `libs.google.oss.licenses` lands on its `google-oss-licenses = …` in
  `gradle/libs.versions.toml` (`libs.plugins.…`, `libs.bundles.…`, `libs.versions.…` in their
  tables), and in Kotlin or Java, `d` on `bookmark_removed` in `R.string.bookmark_removed` lands
  on its `<string name="bookmark_removed">` in `res/values/strings.xml`, with a picker when a
  translation declares it too; `R.drawable.x` and the other file resources open the file, and
  `R.id.x` its `@+id/x` in a layout. (#385)

### Changed

- `d` outside the project answers faster: the dependencies are read in parallel, each file's
  comments are lexed once, and a Python member is looked for in the packages the file imports
  before every installed one, so its picker lists those alone (#318).
- `d` in Java and Kotlin reads the type a cast, a pattern or a smart cast gives the receiver:
  `var m = (Meter) any`, `((Meter) any).reading()`, `any instanceof Gauge g`, Kotlin's `dao as
  NewsDao`, `(dao as? NewsDao)?.purge()`, and `dao.purge()` inside `if (dao is NewsDao)` or an
  `is NewsDao ->` branch of `when (dao)` jump to that type's member, `purge → NewsDao.purge (via
  dao: NewsDao)`, where they offered every `purge` of the project. So do a dotted type,
  `Meter.Dial dial`, and Kotlin's `x?.m` and `x!!.m`; `Map.Entry` says `no definition`. (#388,
  #391)

### Fixed

- `d` on `Settings` in a Java `new Settings(entries)` no longer jumps to `Settings(String path)`
  when the class also declares `Settings(Map<String, Object> entries)`: the comma inside `<…>` of
  a parameter list no longer counts as a second parameter. (#675)
- A `.fs` file is highlighted as F#, not GLSL. (#427)
- A Bazel `BUILD` file is highlighted as Starlark instead of as XML, and a `BUCK` or `.star` file
  is highlighted at all. (#431)
- After `s` or `D` to a Nix name ending in `'`, such as `discount'`, the hint that `d` would have
  got there counts the run, as it does for every other name. (#648)

## [0.8.1] - 2026-10-02

### Added

- `f` folds Python code into one line ending in `⋯`: on a line that opens a construct it folds
  that construct (`def`, `class`, `if` / `elif` / `else`, `for`, `while`, `with`, `try` /
  `except` / `finally`, `match` / `case`, a call, list or dict wrapped over lines, a docstring, a
  run of imports); anywhere else inside a function it folds the function, from a blank line
  between methods the class. `f` on the folded line unfolds it; Up and Down step over a fold, and
  a jump inside one (`:`, `/`, `n`, `d`, `u`, `s`, `[`) opens it. A fold stays with its file
  across jumps and moves with the lines written above it. Other languages say
  `no fold rules for .rs` until they are proven the same way (#623). (#598)
- `d`, `u` and `D` in Nix, where `d` said `no rules for .nix`. `d` on `mkService` in
  `shopLib.mkService` lands on its `mkService = { name, port ? 8080 }:` in `lib/default.nix`; on a
  name a `let` binds or a parameter (`{ config, pkgs, ... }:`, `x:`) names, on that binding in the
  same file, `pkgs: local`, never on another file's `pkgs =`; on a path, `./nginx.nix` or `../lib`,
  the file or the directory's `default.nix`. `D` lists the bindings of functions, and `u` reads
  `my-package` and `x'` as one name. (#430)
- `d`, `u` and `D` in CMake (`CMakeLists.txt`, `.cmake`), where `d` said `no rules for .txt`.
  `d` on a call of `shop_add_library` lands on its `function(shop_add_library name)` in
  `cmake/ShopHelpers.cmake`, whatever case the call is written in; on `${SHOP_WARNINGS}` on its
  `set(`, on `SHOP_TESTS` on its `option(`, on `core` of `Shop::core` on the
  `add_library(Shop::core ALIAS shop_core)` that makes it. `d` on `include(ShopHelpers)`,
  `add_subdirectory(app)` or `find_package(Boost)` opens the file it names, and on
  `FetchContent_Declare` CMake's own `Modules/FetchContent.cmake`, read-only. `D` lists functions,
  macros and targets, and `u` reads `shop-core` and `Shop::core` as one name. (#432)
- `d` in HTML, CSS, SCSS and Less. On a class in `className="btn-primary"` of a `.tsx` file, or
  in `class="btn-primary"` of an HTML file, it lands on `.btn-primary {` in the stylesheet,
  where it said `no definition for btn` or `no rules for .html`; `u` reads `btn-primary` whole.
  A stylesheet's custom properties, SCSS variables, mixins, functions and placeholders, Less
  variables and mixins, and keyframes are found too, a `@use` namespace narrowing them to its
  module, and the path of a `<link href>`, a `<script src>` and an `@import` or `@use` opens
  its file. `D` lists the mixins, functions, placeholders and keyframes. (#415)
- `d`, `u` and `D` in Dart (`.dart`, Flutter included), where `d` said `no rules for .dart`.
  `d` on `formatPrice` lands on `String formatPrice(int cents) =>` in the project; on `get` of
  `http.get` behind `import 'package:http/http.dart' as http` it opens the `http` package's
  `get` in the pub cache, read-only, saying `via import package:http/http.dart`. Classes behind
  their modifiers, mixins, extensions, enums and their values, typedefs, functions, methods,
  getters, setters, constructors (`User.fromJson`) and fields are found; a call statement is not
  taken for a declaration. Outside the project `d` reaches the packages `pub get` lists and the
  SDK. `D` lists `abstract interface class Repo` as `Repo`, and functions and methods too, and
  `_$UserFromJson` is one name. (#414)
- `d`, `u` and `D` in Vue, Svelte and Astro components (`.vue`, `.svelte`, `.astro`), where `d`
  said `no rules for .vue`. Inside a component's `<script>` block (Astro: its frontmatter) every
  TypeScript rule applies: `d` on `formatName` lands on `src/names.ts`, `via import`. From a
  `.ts` file, `d` on `UserCard` of `import UserCard from './UserCard.vue'` opens the component,
  where it said `no definition for UserCard`. In the template, `{{ label }}` lands on the
  script's `const label`, an `item` of `v-for` or `{#each}` on that line (`item: local`), and a
  `<user-card>` tag no import binds on `UserCard.vue`. The template and the `<style>` block
  declare nothing, and `D` lists the script only. `.astro` is highlighted as TSX. (#413)
- `d`, `u` and `D` in Scala. `.scala`, `.sc`, `.sbt` and `.mill` files are one kind with Java and
  Kotlin, so `d` on `Ledger.total(xs)` in a `.java` file lands on the `def total` of `object
  Ledger` in a `.scala` one, where it said `no definition for total`, and `d` in a `.scala` file
  works where it said `no rules for .scala`. It finds `class`, `case class`, `trait`, `object`,
  `enum` and its cases, `def`, `val`, `var`, `type`, a named `given` and a case class's fields,
  behind Scala's modifiers; a match case and Java's `case RED:` declare nothing. `D` lists
  Scala's types, `given`s and `def`s. A declaration-shaped line in a Java text block or a Kotlin
  raw string no longer turns a jump into an offer, and in Kotlin a member of a named `object`
  is that object's: `this.heat` inside a nested `object Kiln` lands on `Kiln.heat`, where it
  jumped to the outer class's `heat`. (#416)
- `d` and `D` in Objective-C. A `.m` or `.mm` file joins C and C++, where it said `no rules for
  .m`, and a header reads `@interface`, `@protocol`, methods by any part of their selector,
  `@property` and `NS_ENUM`: `d` on `findUserWithID` in `[repo findUserWithID:@"42"]` offers the
  header's declaration beside the `.m` file's definition, `self.repository.baseURL` lands on the
  `@property`, and `NSString` on Foundation's `@interface NSString`, read-only, from the SDK's
  frameworks. `D` lists classes, protocols and methods. A C or C++ file reads no framework.
  (#417)
- `d`, `u` and `D` in PowerShell (`.ps1`, `.psm1`, `.psd1`), where `d` said `no rules for
  .ps1`. `d` on `get-shopuser` lands on `function Get-ShopUser`, as names ignore case; on `$Id`
  under a `param(` block it says `Id: local`; on `$script:BaseUri` it finds the assignment; on
  the path of `Import-Module ./Shop/Users.psm1` or a dot-source it opens the file. Classes,
  enums, filters and aliases are found too, and outside the project the module directories of
  `PSModulePath`. `u` and `d` read `Get-ShopUser` as one name, and `D` lists it whole. (#420)
- `d` in Ruby looks outside the project: in the gems `Gemfile.lock` names, at their locked
  versions, in the standard library, and in the core's RBS signatures. `throttle` in a
  `Rack::Attack` initializer lands on rack-attack's `def throttle`, read-only, where it said `no
  definition for throttle`, and `Account.find` reaches ActiveRecord's. The gems are found in the
  project's `BUNDLE_PATH`, `GEM_HOME`, or the Ruby `.ruby-version` names under rbenv, mise, asdf
  or chruby; nothing of the project is run. `mattr_accessor` and `cattr_accessor` declare their
  names as `attr_accessor` does. (#369)
- `d` in JavaScript reads the types JSDoc writes. With `/** @type {ParsedCLIOptions} */ let
  options;`, `@param {ParsedCLIOptions} options` or a function's `@returns {CodePathState}`,
  `options.maxWarnings` lands on the `@property` line of the `@typedef {Object}
  ParsedCLIOptions` that declares it, saying `via options: ParsedCLIOptions`, where it jumped to
  another type's `maxWarnings` found by name. A `@typedef {import("./options").X} X` reads the
  type of that module, and the search by name counts a typedef's `@property` lines as fields.
  (#347)
- `d` in C knows the struct of a receiver from its declaration. In `void f(client *c)`,
  `c->flags` lands on the `flags` of `client`, saying `via c: client`, where it offered every
  `flags` of the project. The type comes from a parameter or a local, a
  declaration at file scope or a global a header declares once (`extern struct redisServer
  server;`), or a function's return type (`lookupClient(x)->flags`), and is followed through a
  chain, `c->bstate.btype` and `c->argv[j]->ptr` included. A receiver not read offers the fields
  of the name as before, and a longer chain says where it broke. (#386)
- `d` in C and C++ knows enum constants and the member functions a class only declares. `return
  GREEN;` lands on `GREEN` in its `enum`, where it said `no definition for GREEN`, and a constant
  in an initializer list is still no declaration. `it->Valid()` offers the interface's `virtual
  bool Valid() const = 0;` beside the overrides, and on an out-of-line `Status
  VersionSet::Recover(…) {` it lands on the declaration in `class VersionSet` instead of offering
  another class's `Recover`. A member declared in its class and defined out of line stays one
  row, the definition. (#373)
- `d` in Java and Kotlin reads the type a receiver's declaration writes: on `line.total()`
  with `Line line` (a parameter, a local, a field, `var line = new Line()`, Kotlin's
  `newsDao: NewsDao` or `val g = Gauge()`), and on `newsDao::deleteAll`, it jumps to that type's
  member or one of a type it extends in the project, `total → Line.total (via line: Line)`,
  where it offered every `total` of the project. A type the project does not declare, such as
  `ArrayList`, Spring's or Compose's, and a string literal say `no definition` instead of
  offering or jumping to a namesake; a project's Kotlin extension on the type still counts. A
  type parameter, a smart cast and a type the rules do not read stay by name, as before.
  (#388, #391)
- `d` in Java on a Lombok accessor, `user.getTitle()` of a `@Data`, `@Value`, `@Getter` or
  `@Setter` class, lands on the field Lombok writes it for, `getTitle → User.title (via user:
  User)`, where it said `no definition`; `User::getTitle` and the classes it extends find it the
  same way. A receiver whose type is not read offers the fields, never jumps to one; a method
  written by hand wins, and a `static` field or `AccessLevel.NONE` gets none. (#381)
- `d` in Ruby reads the Rails DSL: `has_many :followers`, `belongs_to`, `scope :recent`,
  attachments, `attribute`, `enum` and `delegate :email, to: :user` declare the name, and a column
  of `db/schema.rb` is found under its table, `collections.language`. `Account.recent` lands on
  its `scope`, one of a concern's `included do` too; `x.followers` offers the declaration. They
  said "no definition". (#374)

### Changed

- The `?` overlay on a screen narrower than 118 columns wraps an action too long for its row
  onto the next row, under its own column, instead of cutting it at the border (#458).
- `d` in Python reads a dependency's modules as Python imports them (#329). `pytest.fixture` lands
  on `_pytest/fixtures.py` and `pytest.mark` on `MARK_GEN` in `_pytest/mark/structures.py`, saying
  `via import _pytest.mark.structures`, where it offered every `fixture` installed or jumped to a
  namesake in another package: a module that hands a name on through its imports is followed,
  relative and `*` imports too, down to the import line of a compiled source (`StringIO` in
  `io.py`). `json.dumps` is `json/__init__.py`'s alone, never `kombu/utils/json.py`'s, and under a
  `.venv` the base interpreter's pip is no longer searched. A name the imports lead nowhere with is
  offered, never jumped to.
- `d` in Python reads classes declared in dependencies (#340). `self.assertEqual` in a
  `unittest.TestCase` subclass lands on `unittest/case.py`, saying `assertEqual →
  TestCase.assertEqual (via self: T)`, where it said `no definition for assertEqual`; `p.write_bytes`
  with `p: Path` lands on `pathlib`'s, not on a namesake in `anyio`, and `Document.objects` reaches
  the `objects` of a base outside. Bases are followed through the dependency's own imports.
- `d` in C and C++ looks outside the project in the headers the file includes first. On
  `pthread_equal` it lands on `pthread/pthread.h`, one row where the header and a link to it
  were two; `printf` no longer offers gettext's `libintl.h`, nor `s.append("x")` 148 methods
  from headers `s.cc` never includes. A `.c` file reads no C++ header, libc++'s own headers
  (`<string>`, `<mutex>`) are read, and `std::mutex` finds the class and `std::malloc` its
  declaration, where they found none. A word no included header declares is searched by name, as before. (#382)

- `d` in TypeScript and JavaScript finds a member behind an imported qualifier that no
  declaration pattern read: an enum member, `CollectionPermission.Admin` and `ImageStatus.ZOOMED`
  with no value, a static field, `TableCell.presetColors`, and a key of an exported `const`
  object literal, `RateLimiterStrategy.TwentyFivePerMinute`. `env.APP_NAME`, where the module
  exports `new Environment()` as its default, lands on the field of `Environment`, `via env:
  Environment`, and `QUOTE_SETTINGS.backtick` on the key of a `const` literal of the file itself.
  A member of a JavaScript global, `JSON.parse`, is looked for in TypeScript's lib and
  `@types/node` and in what the project declares global (`window.dataLayer` in a `declare
  global`), never among the project's methods. They said "no definition", and
  `JSON.parse` jumped to a project method called `parse`. (#341)
- `d` in TypeScript and JavaScript on a name imported from a barrel of the project that hands it
  on from a package, `export { x } from "lodash"`, lands on `x` in the installed package, as an
  import straight from the package does, or on the import with `(not installed)`. It searched
  the project by name. (#527)
- `d` in Java and Kotlin reads the `import` lines: a class the file imports from the project
  opens in that package's file, `User: via import src/main/java/app/a/User.java`, not a picker
  of every `User`; `import static a.b.C.*` finds `isNull` in `C`; a name imported from outside
  the project, `Objects.equals` or Compose's `stringResource`, says `no definition` instead of
  landing on a namesake; and on the import line a package segment such as `halo` declares
  nothing. A capitalised name the file does not import is its own package's first, then that
  of a wildcard import. The packages are what the files' `package` lines say. (#372)
- `d` in Java and Kotlin reads more of the language: a constructor whose parameters wrap, a
  record's components (`vote.group()`), and the `val`s of a one-line Kotlin primary
  constructor (`s.height`) are declarations; a line inside a Kotlin raw string or a Java text
  block is none. A Java call is never a variable, `values()` lands on its enum, `new Rule(a, b)`
  on the constructor taking two arguments and an overload on the one the arguments fit; and a
  Kotlin infix call, `alias(x) apply false`, finds only an `infix fun`. (#367)
- `d` in Swift reads the type of the value in front of the dot. `lhs.value` in `static func -
  (lhs: Instant, rhs: Instant)` lands on `Instant.value`, `value → Instant.value (via lhs:
  Instant)`, and so does a member on a property (`let encoder: FormEncoder`), on a local made by
  `FormEncoder()` or by a call declaring `-> FormEncoder`, on `self.session.request`, on an
  optional `backup?.encode(…)` and on the element of a loop over `[T]`. On a type from outside
  that the project only extends, `URLRequest` or `Data`, it lands on the extension declaring the
  member, or says `no definition`. It offered every member of the name in the project. A
  protocol, `any` or `some`, a generic parameter, a tuple or a closure stays found by name. (#384)
- `d` in Swift reads a member's kind from how it is written. An implicit member, `.bytes` in
  `case let .bytes(count):` or `.post` as an argument, lands on the enum case or `static` member
  of the name, and in a `case` pattern on a case alone; `Endpoint.method(…)` lands on the
  `static func` and no longer offers the instance property beside it. A bare name in a type's
  body, `lock()` in `extension Lock` or `timeout` in a subclass of `BaseTestCase`, lands on that
  type's member, its extensions' or its superclass's: `timeout → BaseTestCase.timeout (via self:
  DownloadTests)`. It offered every declaration of the name in the project. (#380)
- `d` in Rust on `x.word` lands on the `word` of the type `x` holds, as it does in Python,
  TypeScript and Go: `td.path()` after `let td = tmpdir();` goes to `TempDir::path`, `via
  tmpdir() -> TempDir`, and `self.config.capacity` to the field of `Config`, `via self.config:
  Config`, where both offered every namesake of the project. The type comes from the `impl`
  around `self`, a parameter's or a closure parameter's type, `let x: T`, `T { … }`, `T::new()`
  and a function's `-> T`; `Option<T>`, a generic and a type declared twice stay the search by
  name. (#377)
- `d` in Rust looks where a path's first name says, before any namesake of the project: the
  crate a `use` names or the path spells (`crate`, `self`, `super`, a crate of the workspace, the
  standard library, a crate of `Cargo.lock`), and in it the module the path spells. `File::open`
  behind `use std::fs::File` lands on `File::open` in the standard library, `io` in `io::Result`
  on its `pub mod io;`, a name `use crate::helpers::norm` brings in on `norm` in `helpers.rs`, and
  `std::fs` on the standard library's `fs` rather than every platform's `os/*/fs.rs`. They jumped
  to a project method of the same name or offered every namesake. (#350)
- Every list you filter by typing reads the query as you write it, as `D` has since #293: in
  `o`'s file picker, the theme list and the `d` and `u` pickers, `^`, `$`, `!`, `'` and `\` are
  characters of the name: `!test` finds `!test.md` instead of hiding every path holding `test`,
  and `.rs$` looks for a `$` instead of the end of the path. Spaces still separate words
  matched in any order. (#519)
- `d` in PHP resolves a class name as PHP does, through the file's `use`, else its `namespace`,
  and follows `composer.json`'s PSR-4 map to the file. `Song::query()` behind `use
  App\Models\Song;` lands on `Song::query (via import app/Models/Song.php)`, and
  `AlbumResource::toArray()` on the `AlbumResource` of the file's own namespace, where both
  offered every declaration of the name. A class outside the map is read in `vendor/` first:
  `Arr::get()` behind `use Illuminate\Support\Arr;` lands on Laravel's `get`, not on the
  project's own. In a monorepo a package's file reads the root `composer.json`'s map too, a
  name in a file of several `namespace` blocks resolves in its own block, and `<?php namespace
  X;` on one line counts as the file's namespace. (#351, #579)
- `d` in PHP proves the class of the receiver in front of `->`: `$event->podcast` in
  `handle(UserUnsubscribed $event)` lands on the `podcast` of `UserUnsubscribed`, `(via $event:
  UserUnsubscribed)`, and `$this->artistRepository->getRecentlyAdded()` on the method of
  `ArtistRepository`. The class comes from a typed parameter or property, `new T(…)`, or the
  return type of `T::make()`, `$this->make()` or `make()`. It offered every property or method
  of the name in the project. (#361)
- `d` in Python lands where one answer fits in four more places. A function declared inside
  another and called there is that function's local, `pick → outer.pick (local)`, where `d`
  offered every `pick` of the project. A parameter of a signature wrapped over several lines
  lands on its own line, not on the `def`'s. A `def` nested in a function is no method, so
  `job.run()` no longer offers it beside `Job.run`. A call of an `@overload` set lands on its
  implementation instead of offering every stub; a `.pyi` of overloads only keeps its picker.
  (#338)
- `d` in C# reads the type a receiver is written with: `service.ExtractRedirectUri` after
  `var service = new RedirectService()` jumps to `RedirectService`'s method, through fields,
  properties, parameters, `this`, `base`, casts, patterns and an awaited call, and names the
  links on the status line (`via _uow: Uow → Users: UserRepo`). `Checked = …` in
  `new ScopeViewModel { … }` jumps to `ScopeViewModel.Checked`. A member of a type the project
  does not declare, a MAUI `Label`'s `Text` or an EF `DbContext`'s `SaveChangesAsync`, says `no
  definition` instead of offering or jumping to the project's namesakes, unless the project
  declares an extension method for it. (#352)
- `d` in C# looks only in the projects a file compiles against: its own `.csproj` and the
  projects that one references, `<ProjectReference>` items of a `Directory.Build.props` above it
  included. In a solution of many projects, `new Address(…)` jumps to its own project's `Address`
  instead of offering another project's namesake beside it. A file under no `.csproj` is seen
  from everywhere; a script, two projects in one directory, a source file a `.csproj` pulls in
  from outside its directory, a shared project and a reference that cannot be read keep the
  whole repository in sight, as before. (#349)
- `d` in C# reads a name as C# resolves it. A type where only a type can stand, `Buyer buyer`,
  `new Address(`, `List<Buyer>`, `(Buyer)x` or a base in a class header, jumps to the type, not
  to a property or a constructor named like it. A bare name in a class lands on what the class,
  a `partial` part of it or a base it names declares, `IsBusyFor → ViewModelBase.IsBusyFor (via
  OrderViewModel)`, before any namesake, and another class's member is never offered for it. A
  segment of a `using` or `namespace` line offers the project's namespaces of that name and
  nothing else, `no definition` when there are none. An overload that cannot take the call's
  number of arguments is not offered. A constructor whose body stands on its line or whose
  parameters wrap counts as a declaration, and a use of the type lands on the type, not on it.
  They offered every declaration of the name. (#360)
- `d` in Swift offers only what the compiler sees from the cursor. A file of the library no
  longer offers a declaration of a test target (`.testTarget(` in `Package.swift`, or `Tests/`
  with no manifest), another file's `private` or `fileprivate`, a type declared inside another
  function, or a nested `PathMonitor.Result` for a bare `Result` outside `PathMonitor`: `d` on
  `Result<Int, Error>` says `no definition for Result` where it jumped to the nested one. A
  generic parameter binds its name, so `Value` in `struct StreamPublisher<Value: Sendable>`
  lands on that header, `Value: local`, where it jumped to a namesake inside a test. Inside a
  function nested in another, the outer function's locals and parameters are found too. (#375,
  #564)

### Fixed

- `[` and `]` bring back the screen you left: the line you jumped from stands on the same row it
  stood on, where it came back to the middle of the screen. (#621)

- In `--review`, `p` on a Markdown file of the review shows it rendered, where it said `in review`:
  a file the branch adds or changes as it stands now, a deleted one as it was, without the diff's
  marks; `p` again shows the source with its diff. A file shown rendered that the branch comes to
  change stays rendered; a file of the review opened again shows its source. (#596)

- `d` in Python showed each declaration twice when `sys.path` lists a directory and its
  `site-packages` both, as a pyenv, uv or python.org `python3` does without a `.venv`. (#329)

- `d` in C reads no declaration inside a multi-line `#define`, and a one-line `typedef struct
  client { int flags; } client;` declares `client` alone: `return flags;` and `return args;`
  jump to the global they read instead of offering a macro's parameter or a struct's field.
  (#382)

- `d` in TypeScript and JavaScript on an arrow function's parameter itself, `crate` in
  `crates.map(crate => 0)`, offers a namesake elsewhere under "at a declaration", or stays on
  the line when there is none. It jumped to a module-level `const` of the same name. (#534)
- `d` in Kotlin names the members of a class whose header wraps over lines, `class Repo @Inject
  constructor(` … `) : Base {`, and of its `companion object` as those of a one-line header:
  the status reads `getTopics → Repo.getTopics` and `DEFAULT → Repo.DEFAULT`, and `Repo::m`
  and `Repo.DEFAULT` find them. The walk up to the class stopped at the `) : Base {` line and at
  `companion object`, so those members had no class in their name. (#523)
- `d` in Swift on a function's parameter in its header, as `attempt` in
  `func probe(_ attempt: Int) {`, answers as on any declaration: the line itself, or the
  namesakes offered under "at a declaration". It jumped to a lone namesake elsewhere. A name a
  `for`, an `if let` or a closure binds over an outer one of the same name answers the same way;
  it jumped to the outer one. (#533)
- `d` on a key of a Rust struct literal, `hyperlink` in `Printer { hyperlink: 1 }` or `Self {
  hyperlink: 1 }`, lands on the field `hyperlink` of `Printer`, the project's or a dependency's.
  It jumped to a method `hyperlink()` of the same name, or said "no definition". (#529)
- `d` no longer crashes merl on a line where a non-ASCII character stands next to a name: in
  Rust inside the brackets of a call whose name ends in a combining accent (`méthode(42)` as
  macOS spells it), in PHP on a namespaced name after a `©`, and in C or C++ inside a block
  whose header holds non-ASCII names on both sides of a bracket. It panicked, which left the
  terminal unusable until `reset`. (#543)
- `d` in Go on a parameter's type named like the method it belongs to, the second `Send` of
  `Send(msg Send) error`, jumps to `type Send`. It offered a picker of `type Send` and every
  method `Send` beside it. (#536)
- `d` in Ruby on a method written `def User.build` inside `class User` names it `User.build` in
  the status line, and its parameter `User.build.arg`: the class was named twice,
  `User.User.build`. (#535)
- `d` in C# on a constant after `is`, `Max` in `x is Max` with a `const int Max`, lands on the
  constant when no type of that name is in sight. It said `no definition for Max`. (#581)
- `d` in Swift on a call of a local function or type, `vent()` under `func vent() {}` declared
  inside the method, lands on that local declaration, `(local)`, from the method's body and from
  a function nested in it, whether it is declared above the call or below; two local overloads
  are offered together. It jumped to the type's member of the same name through `self`, or
  offered both. (#577)

## [0.8.0] - 2026-10-01

### Added

- In `--review`, `s`, `D`, `u` and `d` find the code the branch deleted as well as the files on
  disk. A deleted line in their lists is a row like any other, told apart by the gutter's `▎`
  in red, as an added one is by its green, and Enter lands on it. `d` on a deleted line reads
  the code as the base had it: it opens the function the call called, on its red lines when the
  branch deleted or rewrote it, on its line now when the branch kept or only moved it, and `[`
  goes back. On any other line `d` looks in the branch's code first and opens a deleted
  definition only when the branch has none. (#440)
- `d` in Go follows a table test: in `for _, tc := range []struct {…}{…}` and in a range over
  `tests := []struct {…}{…}`, `tc.name` lands on the field of the struct written in place,
  `name → struct{…}.name (via tc: struct{…})`, and so does the key `name:` of an element of
  the table. It read the loop from its `}{` line, so `tc` was not bound and `tc.name` jumped to
  a namesake elsewhere or offered every `name` of the project. (#330)
- `d` in Go proves a receiver whose type the standard library or a `go.mod` module declares:
  `wg.Add` on a `sync.WaitGroup`, `t.Errorf` through the `common` a `testing.T` embeds,
  `ctx.Err()` on the `context.Context` interface, `r.URL.Path` through `*http.Request`, `srv`
  from `httptest.NewServer(…)`, and a key of `sync.Pool{New: …}`, each read-only in GOROOT or
  the module cache. It offered every method of the name in GOROOT and the module cache, often
  hundreds and often without the field, and took a third of a second or more for it. (#334)
- The hidden characters a file can hold are on screen, in every file and in `--review`: the
  bidirectional controls behind "Trojan Source" (U+202A–U+202E, U+2066–U+2069, U+200E, U+200F,
  U+061C) and the zero-width U+200B, U+2060 and U+FEFF are drawn as their code, such as
  `<202e>`, on an amber of their own that no diff uses. The cursor steps over one in a press and
  Delete removes it; a ZWJ inside an emoji and a BOM at the start of a file stay as they are.
  (#401)
- `merl --reviews` prints your review sessions of the last 30 days, newest first: the branch,
  which round of it the session was, the files, hunks and lines under review, the active time
  and how much of it was on the review's files, and the excursions, the jumps with `d`, `u`, `D`,
  `s` or `o` from a file of the review out of it; then the median time of a first round and of a
  later one. merl notes every `--review` session in `~/.local/state/merl/reviews.tsv` on exit,
  next to `keys.tsv`, and shows nothing while you review. Only the time between presses counts,
  a gap longer than five minutes as five, so a review left open over lunch is not an hour of
  review; a session closed with `q` and nothing else is not noted, a `git switch` during a review
  ends its session and starts one for the branch now checked out, sessions older than 30 days
  are dropped, and `--tutor` and `--drill` note nothing. (#242)
- `p` shows a Markdown file rendered, in place of its source, and `p` again shows the source, at
  the same place both ways; Enter in the preview edits the source where it stands. Headings,
  emphasis, lists and task lists, quotes and GitHub alerts, tables in box drawing aligned as
  their `:---:` says, code blocks in the theme's colours, rules and footnotes render; links show
  their text, images their alt text. The preview is rendered from the open buffer, so a file an
  agent rewrites renders again, and it reflows to the pane: a table wider than the pane narrows
  its widest columns and wraps inside their cells instead of falling apart. Reading keys move a
  cursor row; `/`, `v`, `d` and `u` have no word or column to act on there and do nothing. In
  `--review` the diff stays on the source: `p` on a file of the review says `in review`. The
  tutor has a Markdown plan and a lesson for it. (#249)
- Short forms of the launch flags: `-r` for `--review`, `-b` for `--base`, `-t` for `--theme`,
  `-d` for `--drill` and `-k` for `--keys`. `merl -r feature -b origin/dev` is
  `merl --review feature --base origin/dev`. (#238)
- Inside a long function whose first line has scrolled off, that line stays pinned on top of
  the code with its line number, on a band of the cursor line's colour, so the screen always
  says which function this is: in a review, where `c` drops you in the middle of one, and
  everywhere else. A method pins its `impl` or `class` too, two lines at most; a loop or an `if`
  pins nothing. The code starts under the band, so no line hides behind it, and the pages,
  Ctrl+D and Ctrl+U move by the rows left under it. (#248)
- A line longer than merl draws (20 KB: a minified bundle, a one-line JSON dump) ends in a dim
  `…` right after its last drawn character, wrapped or not, so a cut line never reads as whole.
  (#283)
- In `--review`, `u` and `s` mark each row on a line the branch added or changed with the
  gutter's `▎`, in its colour, and leave an untouched line's row blank, so the readers a change
  did not reach stand out; the rows keep their order. `o` lists the review's files first, each
  with its panel letter, then the rest of the project as before. `review_list_marks = false` and
  `review_open_files_first = false` in `~/.config/merl/config.toml` turn either off. (#246)
- A macOS Intel binary, `merl-x86_64-apple-darwin.tar.gz`, ships with each release. (#394)
- `d` in Markdown (`.md`, `.markdown`, `.mdx`) follows the link under the cursor, on its text or
  its target: `[README](../README.md#languages)` opens `README.md` at its `## Languages` heading,
  `[below](#setup)` goes to a heading of the same file, `#L12` to a line, and a reference link
  goes through its `[label]: target` definition. A code span naming a file of the project,
  `` `src/search/kind.rs:30` ``, opens it; a bare name several files carry is a picker of them.
  A missing file or heading says so, and a link in a code block or a comment is not followed.
  `D` no longer lists the examples of a README's code blocks as declarations. (#421)
- GraphQL: `d` in a `.graphql`, `.graphqls` or `.gql` file lands on a `type`, an `interface`, an
  `input`, an `enum`, a `union`, a `scalar`, a `directive`, a fragment from its `...spread`, a
  named operation, a field (`email → User.email`) and an enum value; on the path of an
  `#import "./parts.graphql"` it opens that file. `extend type`, a selection, an alias, an
  argument and a `$variable` are no declarations. `D` lists the types, directives, fragments and
  named operations. (#419)
- `d`, `u` and `D` in Protocol Buffers (`.proto`): `d` finds a `message` (a nested one too), an
  `enum` and its values, a `service`, an `rpc`, a field and a `oneof`; `d` on the path of an
  `import` opens that file; a type qualified by its package, `billing.v1.Money` or
  `google.protobuf.Timestamp`, lands in that package's files, and the well-known types `protoc`
  installs are reached, read-only. `D` lists the messages, enums, services and rpcs. (#418)
- `d` in C and C++ on a parameter or a local lands on its declaration, `i → crc32::i (local)`,
  and it hides every function, macro, global and system header of the name: `link` in
  `link->node` jumped to POSIX `link()`, and a variable declared a few lines up said `no
  definition`. The innermost block that declares the name wins, a block closed before the cursor
  does not count, a `for (int i = …)` binds `i` in its loop, and a lambda reads on into the
  function around it; `a && b == c`, `x & FLAG` and a declaration inside a string bind nothing.
  In a C++ method a bare member, `return filename_;`, lands on its class's field. A value, a word
  followed by `->` or `.`, is never a struct, a `typedef` or a `using` alias, so `group->pel`
  no longer opens the system's `struct group`. (#378)

### Changed

- `d` in Java and Kotlin looks in the scope around the cursor first. A parameter, a lambda's
  parameter, a loop variable and a local of the blocks around it answer as `(local)`, where `d`
  said `no definition` or offered namesakes from other files: `directionParams →
  SortUtils.resolve.directionParams (local)`. A name the method's class declares, a field or a
  method, the members of its `companion object` and the properties of its primary constructor
  included, answers `via` the class, `scheduler → Use.scheduler (via Use)`, and one the class it
  extends declares, `via` that class, before the declarations of the name in the rest of the
  project. Kotlin's `it` and the implicit receivers of `with` and `apply` are not read. (#376)
- `d` in PHP on `$this->name`, `self::NAME`, `static::name()` and `parent::name()` reads the
  class the cursor is in, the traits it uses and the classes it extends, and lands on the one
  declaration, `open → BaseStorage::open (via $this: ImageStorage)`, where it offered every
  declaration of the name in the project: a picker for most, and a jump to another enum's case
  for `self::INVALID`. A parent from `vendor/` is read from its own file, and a member it does
  not declare is `no definition` rather than a namesake of the project. (#356)
- `d` in TypeScript and JavaScript reads fewer calls as declarations and finds more real ones.
  A call that passes a callback, `it("works", async () => {`, or wraps after its `(` and closes
  with `);` is no method, so `d` on it no longer says `at a declaration` over thousands of
  namesakes, and `type NodeSpec,` in a wrapped import list is no type alias. An optional method
  signature wrapped over lines, `onCodePathEnd?(` over `): void;`, and the fields of a class
  whose header wraps at a type argument, `implements Base<{` over `}> {`, are found by name.
  (#343)
- `d` in TypeScript and JavaScript on a bare name that the file declares jumps there, `local`,
  instead of opening a list of every namesake in the project: a `type Props`, a `function
  report` inside a rule's `create`, a `class Config`. A declaration at the top of the file counts
  wherever it stands, so a styled `const Container` at the bottom of a component is found from
  above it. Several declarations of the name in one scope, such as an `interface` beside a
  `namespace`, are a list of those alone. (#337)
- `merl -r BRANCH` reviews a branch another worktree has checked out, an agent's say, in that
  worktree, as `merl -r` started there would: nothing is fetched, switched or reset there, and its
  work not committed yet is part of the review. Before, merl exited with git's `already used by
  worktree`. (#396)
- In `merl --review` the lines the branch deleted are lines of the text, as they are on a GitLab
  or GitHub diff page: the cursor stands on them, and every move, Up, Down, the pages, `{` and
  `}`, Home and End, the words, goes through them as through the file's own lines, so a deletion
  taller than the pane is read line by line and the lines deleted at the end of a file are
  reached with Down. Shift+moves and `v` select them, Ctrl+C copies them as they were, and `/`
  finds text in them. `c` and `C` stand on the first line of a change, its first deleted line
  when it starts with a deletion. On a deleted line the status bar reads its number in the file
  the branch started from, negative: `-9:5`. Nothing edits a deleted line: typing on one, or on
  a selection that holds one, says `deleted`.
  `:12` and the gutter still count the branch's lines. For the selection to show on the red
  tint, eight themes take a selection colour a shade further from their background, in every
  file: rose-pine, rose-pine-moon, melange-dark, bamboo, cendre, ayu-light, jellybeans-light and
  neomodern-light. (#439, #279, #296, #408)
- `merl --review feature` reviews `feature`: the branch goes after a space, as the base does after
  `--base`, and `--review=feature` still works. A file after a bare `--review` is now read as the
  branch: `merl --review` opens on the first hunk, and `o` opens any file. (#238)
- `merl -r origin/feature`, the name as `git branch -a` or a merge request shows it, reviews
  `feature` as `merl -r feature` does, where git refused to switch to a remote branch. origin
  without that branch is an error, `merl: no branch feature on origin`, even when a local
  `feature` exists; offline, the local branch opens with `origin/feature not fetched`. A local
  branch literally named `origin/feature` is still that branch, and other remotes' prefixes are
  part of a local name, as before. (#271)
- `merl --review` paints the diff as GitHub does: the lines the branch deleted on a red tint,
  in their syntax colours instead of grey, the lines it added on a green one, and on a changed
  line the words that changed on a stronger tint, on the old line and on the new. A deleted line
  wraps like the text instead of ending in `…`. Where a block deletes a different number of lines
  than it adds, as when a comment goes in above the changed line, the lines still pair by what
  they have in common; GitHub and GitLab paint nothing there. The colours come from each theme's
  background, so every theme, a user's own included, has them. (#165)
- `d` on `Type::name` of a Rust type the project declares more than once lands in the type the
  file's `use crate::…` or `use super::…` names: `Cache::new` behind `use crate::store::Cache;`
  is the `new` of an `impl Cache` in `src/store.rs` or `src/store/mod.rs`, `via import`, where
  it was a picker of every `new` in the project, `by name`. Only a `use` at the top of the file
  counts, and a type of the name in the file itself, a name bound twice, a binary under
  `src/bin/` or a module with no `impl` of the type stay `by name`. (#227)
- `d` in C# on `Offer.Cut`, where `Offer` is an `enum` the project declares, lands on `Cut` in
  the enum's body, `via Offer`, where it found nothing: one member per line or several on one,
  with a value or an attribute. The enum has to be in a namespace the file sees, by its own
  namespace, a `using`, a `global using` of its project or the path written out
  (`Shop.Pricing.Offer.Cut`); a bare `Cut` still has no rule. (#466)
- `d` on a bare name in Rust lands on the item the file declares under it, `helper: in this
  file`, where it was a picker of every file's namesake: Rust sees another file's items only
  through a `use` or a path. The item counts where the cursor sees it: a `fn` nested in the
  function, the inline `mod` around the cursor or the file's top level, and the file's own items
  inside a `mod tests { use super::*; … }`. A name the function binds before the cursor, as a
  `let`, a parameter or a closure's, and a name a `use` imports stay as before. `Type::new` where
  the file declares `Type` and another crate a `Type` too lands in this file's
  `impl Type`, `via Type`. (#363)
- `merl --review` keeps the files marked viewed from one start to the next, per branch and base,
  in the repository's git directory, so a review in a worktree has them too. A file changed since
  it was viewed, on screen or between two starts, loses its tick, as on GitLab. A file that leaves
  the review for a while, as during a rebase stopped on a conflict, has its mark back when it
  returns. A review started on a detached HEAD, outside a rebase, keeps its marks only while it
  runs. The marks of a review untouched for 30 days are forgotten. (#240)
- The file panel of `merl --review` dims the line counts and `bin`, so the name reads first, and
  the bottom border gives the size of the branch, `3 files · +13 −1`. A long name of wide
  characters is cut to fit instead of pushing the counts off the panel.
  `review_panel_colours = false` in `~/.config/merl/config.toml` turns the dimming and the totals
  off. (#250, #450)
- On a file the branch did not change, the status bar of `merl --review` drops `hunk 0/0  file
  -/8` and reads as it does outside a review. (#286)
- `merl --review` folds a generated file, as GitHub and GitLab fold it: `c` stops on it once and
  shows a box instead of its text, with its name, its hunks and `+ −`; the next `c` goes on to
  the next file and ticks it viewed, and `C` comes back to it. Enter loads its diff, and `c` then
  walks its hunks; it stays loaded when the branch is reviewed again. Folded is what both forges
  fold: lock files such as `package-lock.json`, `pnpm-lock.yaml`, `poetry.lock`, `uv.lock`,
  `Cargo.lock` and `composer.lock`, `node_modules/`, minified scripts and their source maps, Go's
  `// Code generated … DO NOT EDIT.`, and paths `.gitattributes` marks `linguist-generated` or
  `gitlab-generated`. (#243)
- Enter in a list whose query matches nothing does nothing, as in VS Code's quick open: `o`,
  `D`, `T`, `s` and the lists of `d` and `u` stay open with the query, so Backspace fixes a typo
  instead of the query being lost. Esc still closes the list. `s` no longer closes on such an
  Enter with `no results for …`. (#288)
- With no file open, on the start screen and at the tutor's first lesson, the status bar no longer
  shows a cursor position: `demo/  [tree]` instead of `demo/  1:1  [tree]`. (#285)
- `d` in Java and Kotlin on an enum constant, `Offer.CUT`, lands on the constant in the body of
  the enum, `CUT → Offer.CUT (via Offer)`, instead of `no definition for CUT`, when the project
  declares one type of that name and it is an `enum`. (#457)
- A binary file (an image, a `.pyc`, a build artefact) opens on an empty pane with one centred,
  dimmed line, `binary file, not shown`, and no line number or cursor, instead of the text
  `binary file` as if it were the file's line 1. The status bar still says `read-only`. (#287)
- `d` in Python on a builtin says `next: builtin, no source` and stays put: a bare `next`, `map`
  or `ValueError` that nothing in the file binds, and a member of a value proven to be a builtin
  type, `replace: builtin, no source (via render() -> str)`. It opened a picker of every method
  of the name in the dependencies, or jumped to the only one, after a grep of all of them. A bare
  name nothing binds is no longer looked for among the methods outside either: only at the top
  of a module the file imports with `*`. (#336)
- `d` on a named argument lands on the parameter it names: `Basket(tariff=…)` on `__init__`'s
  `tariff`, `RefreshWindow(interval: 30, maximumAttempts: 1)` on the `init`'s `maximumAttempts`,
  `new self(name: …)` on the constructor's promoted `$name`, and a key of an object passed to a
  function, `<Tag size={2}>` or `const x: Opts = { weight: 1 }` on the key the parameter or the
  type declares. A callee with several declarations offers their parameters in a picker. (#316)
- `d` in Swift on a parameter, a closure's parameter, a `let` or `var` of the function you are
  in, or a name that `if let`, `guard let`, `while let`, `for`, `catch` or a `switch` case binds
  lands on where it is bound, `(local)`, as in Python, TypeScript and Go; a bare `catch` lands on
  its line for the implicit `error`. It searched the project by name: a picker of other types'
  properties and other functions' locals, or with one namesake a wrong jump, as a loop's
  `attempt` to a struct's property `attempt`. (#366)
- `d` in Ruby lands on a parameter, a block parameter or a local of the method you are in, as
  `user → SessionsController.user (local)`. It offered every method's `user = …` in the project
  or jumped to one. A call with no receiver, and `self.name`, lands on the method of the class it
  is made in, then of the modules the class includes and of its superclasses, before a namesake
  of another class: `track → SessionsController.track (via SessionsController)`. (#365)
- `d` in C# on a parameter, a lambda's parameter, a `foreach`, `for`, `catch` or `using`
  variable, an `out var`, a pattern variable or a local lands on its binding in the method you
  are in, `options → Refunds.Register.options (local)`, as in Python, TypeScript and Go. It
  searched the project by name, so it jumped to another method's local of the same name, or
  offered a picker of them, and a parameter had no definition. A primary constructor's
  parameters bind across the type's body, and a lambda's parameter only inside its lambda. (#345)
- `d` in Rust on a parameter or a local lands on what binds it in the block around the cursor,
  `config → Printer::hyperlink::config (local)`, where it jumped to a function of the same name or
  offered every file's `let` of it: a `let` and its patterns, an `if let`, a `while let`, a `match`
  arm, a `for`, a closure's parameters and the function's, wrapped over lines too. The nearest
  binding above the cursor wins, as Rust shadows: in `let x = x.trim();` the right-hand `x` is the
  earlier one. Another function's `let` is no longer offered for a name. (#353)

### Fixed

- `d` in Swift finds a function or an enum case declared with its name in backticks, as
  ``func `default`()`` or ``case `open` ``: it said "no definition", or jumped to a namesake
  elsewhere. (#463)
- `d` on a PHP `$variable` lands on its parameter or its assignment in the function you are in.
  It searched the project by the bare name, so `$weigh` jumped to a function `weigh()` and
  `$courier` to another method's local; a closure's `use (…)`, a `foreach` or `catch` target and a
  destructuring count too, and at the top of a file the file's own assignments. (#464)
- A find match keeps the text's own colours on its tint, as in VS Code. In a theme that tints
  matches without naming their text colour (github-light, vscode-light and -dark, koda, pencil
  and eight more) the matched letters were drawn in the background colour, 1.1:1 to 2:1 on the
  tint, and could not be read. (#480)
- `d` in Lua on a parameter or a `local` of the function you are in lands on it, `(local)`,
  where it jumped to a function of the same name in another module. A `local` inside another
  function, or behind a dot, is no longer offered, and a table key is not read as the local of
  its name. (#461)
- `d` in Elixir reads a name with its trailing `?` or `!`: on `ship!` it finds `def ship!` and
  not `def ship`, and on `Jason.encode!` no longer jumps to the project's own `def encode`. A
  qualifier behind an `alias` (`W` of `alias Shop.Warehouse, as: W`, `Tariff` of
  `alias Shop.Pricing.{Tariff, Coupon}`) and a module written out in full, a `defprotocol`
  included, lead to that module's function instead of a picker of every namesake, and a call
  such as `Shop.currency()` no longer offers the module attribute `@currency`. (#459)
- An edit key with nothing to take does nothing: Alt+Delete at the end of the file, Alt+Backspace
  at its start, Ctrl+X on an empty only line, an empty paste. Each was an undo step that changed
  nothing, so the next Ctrl+Z seemed to do nothing, and it cleared what Ctrl+Y would redo. (#455)
- Ctrl+Y after undoing a Tab over several lines puts the cursor where the Tab left it. It stood at
  the end of the last indented line. (#456)
- In a GitLab CI file, `d` on the value of `stage:` lands on that stage in the file's `stages:`
  list, flow or block form, and says `no definition` in a file without one. It jumped to the job
  named after the stage, `build:` for `stage: build`, which is how most pipelines name them.
  (#473)
- `d` in Java and Kotlin no longer lands on a local of another function or a `private`
  declaration of another file: `Modifier.height` jumped to a `val height` inside some function
  elsewhere, and `isBlank(s)` to a `private static` method of another class. A local answers
  only below it in its own block, never behind a `.` or a `::`, and a `private` declaration only
  in its own file; when one declaration is left that way, it is offered in the list rather than
  jumped to, since it was still found by name only. (#357)
- `d` in Java and Kotlin reads `Type::method` as `Type.method`: `Inner::getName` jumps to the
  `getName` of `Inner`, and `this::show` or `this.show` to the `show` of the class around the
  cursor, where they listed every method of that name in the project. A Kotlin extension is
  named by its receiver, so `Topic::asExternalModel` jumps to `fun Topic.asExternalModel()`
  among the extensions of other types, and `val Topic.testTag` declares `testTag`, no longer a
  second `Topic`. (#362)
- `d` in PHP reads `$x->name` and `$x?->name` as the member they are: a call finds the
  methods of that name, anything else the properties, in the project and then in `vendor/`, the
  `@method` and `@property` tags of a class's docblock included. It searched the bare word, so
  `$join->where(…)` on a Laravel query landed on a local `$where = […]` of an unrelated class,
  `$request->input(…)` on a property, and `vendor/` was never read. A chain broken before its
  arrows reads as one line, and a `.`, which concatenates in PHP, is no member access. (#348)
- `d` in PHP finds a typed class constant, `private const int LIMIT = 500;`, where it said
  `no definition`, and the `@property`, `@property-read`, `@property-write` and `@method` tags
  of a class's docblock, the way Laravel declares Eloquent columns: `$song->title` lands on the
  tag, `title → Song::title`. A `namespace …\Support;` line no longer answers `Support` in
  `use Illuminate\Support\Facades\Route;`, nor a class called like its last part: a segment
  of a qualified name finds only the namespace written up to it. (#344)
- `d` in Go finds a name declared inside a grouped `const (`, `var (` or `type (` block: an
  iota enum, `time.Hour`, `http.StatusOK`, a type of a `type (` block. It said `no definition`,
  or jumped to a namesake elsewhere. A field of a struct inside the block, and a `var (` block
  inside a function, still declare nothing of the package. (#326)
- `d` in Go on a key of a composite literal, `Address` in `Order{Address: addr}`, lands on the
  field of the literal's type, `Address → Order.Address (via Order{…})`, also for an element
  whose type is elided (`[]Item{{Name: "a"}}`) and a type of another package. It looked the key
  up as a bare name and jumped to a namesake type, method or function. A map's keys stay values;
  a literal whose type is not read (an anonymous struct, a type outside the project) offers what
  the name finds and never jumps to one. (#327)
- `d` in Go reads the locals and parameters above a label: gofmt writes `scan:` at the left
  margin of a function, and the scope walk took it for the function's end, so a local used
  below it gave a namesake from elsewhere or `no definition`. And on a name that
  `n, err := second()` declares again in its block, `d` lands on the first declaration, which
  the `:=` reuses, where it offered both lines. (#330)
- `d` in Go looks a bare name up where Go does: a local, a name of the file's own package, of a
  dot import, or a predeclared one (`len` lands in GOROOT's `builtin/builtin.go`). `pkg.X` is
  looked for in `pkg`'s directory only, since Go has no re-exports. It searched every package
  of the project, GOROOT and the module cache, so a name it missed jumped to a namesake of
  another package, a method or another function's local; now that is `no definition`. A
  `package x_test` file no longer sees the names of `package x`. (#332)
- `d` in TypeScript and JavaScript no longer offers another file's function locals: a
  `const`, `let`, `var`, `function` or `class` inside a function, a method or a block is out of
  sight there, in the project and in the dependencies. `Object.values(o)` jumped to a `const
  values` inside some other function, and `node.callee` to a `const callee` of another file. A
  method named by a string or a computed key, `"NewExpression:exit"(node) {`, binds its
  parameters, so `node` there is the parameter. (#339)
- `d` in TypeScript and JavaScript follows a barrel to the declaration: `import { Group } from
  "./models"`, where `models/index.ts` says `export { default as Group } from "./Group"` or `export
  * from "./helpers"`, lands on the class in `Group.ts`, where it fell back to the search by name
  and opened a list of every namesake, or jumped to the wrong one. A module that imports a
  default and exports it again, `export default Text;`, is followed to the module that declares
  it, where `d` stopped on that line. (#335)
- `d` in TypeScript finds a class or an interface whose type parameters prettier wrapped,
  `class User extends Model<` over its type arguments over `> {`: it was dropped as a wrapped
  call, so `d` said `no definition` or jumped to the one namesake left, a client-side model for
  the server's. The return type of an arrow, `): Node => ({`, is no longer read as its
  parameter, which hid the import of `Node`. (#331)
- `d` in JavaScript reads a `require` as an import. A name that `const { helper } =
  require("./m")` or `const Segment = require("./seg")` binds leads into the required module,
  where it stopped on the `require` line; `utils.helper` behind `const utils = require("./m")`
  finds `helper` there, `Segment.make` the `make` of the class that `module.exports = Segment`
  hands out, and `d` on `utils` itself lands on its `module.exports =` line. A `const` continued
  over several lines binds every name it declares, where the names after the first were found
  nowhere. `require("debug")("app")` returns something else and stays a local. (#328)
- `d` in Rust on `x.method()` where the type of `x` is not known lists the methods of that name
  in the project, the standard library and the dependencies the cursor can reach, the traits'
  first, where it jumped to a lone project namesake: `v.unwrap()` on an `Option` landed on a
  private `unwrap` of another crate, `n.clone()` on the project's `impl Clone for Error`, and
  `s.to_string()` said `no definition`. When every candidate is the method of one trait or an
  `impl` of it, `d` jumps to the trait's method, `clone → Clone::clone (via trait Clone)`. (#358)
- `d` in Python and Go follows a typed chain through a type whose module or package has a
  line shaped like its declaration inside a docstring or a raw string: `self.tariff.rate()`
  jumps to `Tariff.rate`. That line counted as a second declaration, so the chain broke and
  `d` offered every `rate` of the project. (#453)
- `d` in Ruby reads names as Ruby does. `empty?`, `save!` and the setter `name=` of `x.name = v`
  are methods of their own, so `fetch` no longer lands on `def fetch?`. `Const.meth` is a class
  method: `def self.meth`, a `def` in `class << self`, in an `extend self` or `module_function`
  module, or in the `class_methods` of a concern the class includes; `Const.new` finds
  `initialize`, and an instance method of the class, or a method of another class by name, is
  never the answer. `class A::B` declares `B`, not `A`, `A::B` in code is a path, and a
  superclass right of `<` is a use of the name. (#387)
- `d` in Ruby on a method of a value, such as `logger.info` or `items.each`, offers the one
  method of that name the project declares in a picker instead of jumping to it: the core and the
  gems are not read, so it may be theirs, and was in most of such jumps in a real project.
  `Const.meth`, `self.meth` and a bare call still jump. (#390)
- In `merl --review`, `c` or `C` after a `d`, `u` or `s` into a file the branch did not touch goes
  back to the hunk you left, and the next `c` goes on from there. It opened the first file of the
  review (`C` the last), and the way back was one `[` per jump. The hunk is found again by its
  place among its file's hunks, so an agent moving it meanwhile does not lose it. (#239)
- Enter on a row of `d`'s list, of `u`, `D` or `s` puts the cursor on the word the row is
  about, as a jump with one match does: the declared name for `d` and `D`, the use for `u`, the
  start of the hit for `s`. It went to the start of the line, so the next `d` or `u` asked
  about `def` or the indent until the cursor was walked onto the name. (#236)
- The status bar names a file outside the project from the root it came from, as the `d` picker
  does: `json/__init__.py` instead of the whole path to the interpreter. A Rust crate or a Go
  module keeps its own name there and in the picker, `serde-1.0.200/src/lib.rs`, so it never
  reads like the project's `src/lib.rs`. A path still too long for the pane is cut from the left,
  `…/json/__init__.py`, so the column, `read-only` and the reason `d` gave stay in sight; in a
  120-column pane they fell off the edge. (#235)
- `merl --tutor` and `merl --drill` no longer save a theme picked with `T` and Enter to
  `~/.config/merl/config.toml`: the theme lasts the session. A learner who pressed Enter in the
  task on Esc lost their own theme. (#278)
- Enter in the `T` picker on a theme of your own that does not load keeps the picker open with
  the load error and saves nothing. It saved the name, and every start after exited on the broken
  file. A configured theme that fails at start now names `config.toml` and the theme set there
  beside the cause, so the way back is in the message. (#277)
- In a line longer than merl draws (it shows the first 20 KB of a minified bundle or a one-line
  JSON dump), the cursor stays on the drawn part: `End`, `Ctrl+End`, the arrows, the word moves,
  `/` and a jump from `d`, `u` or `s` stop where it ends, and `Right` there goes on to the
  next line. They put the cursor at the real end of the line, past everything on screen, with a
  column in the status bar no drawn char had. (#284)
- `u` on a name with a hyphen in a Makefile, Terraform, a Dockerfile or YAML lists that name
  only: `db-main` no longer lists `db-main-2` or `db-main-replica`. In code, where `db-main-2` is
  a subtraction, the line is still listed. (#281)
- Ctrl+X on the last line of a file takes the line with its break, as on any other line and as in
  VS Code: the cursor goes up a line, at its column. It left an empty line behind. Ctrl+C and
  Ctrl+X on the last line copy it with its line break, so it pastes as a whole line like every
  other line's copy. A file of one line is still left with one empty line. (#282)
- `d` in Python on a name an import binds to a module of the project opens that module:
  `views` in `from shop import views`, in `import shop.views` or in `views.index`, `shop` in
  `import shop`, and `from . import views` too, a package at its `__init__.py`. It said `no
  definition for views`, or jumped to a function called `views` elsewhere. A comment inside a
  bracketed import, `a,  # noqa: F401`, no longer hides the name after it, so `d` on that name
  goes through the import instead of offering its namesakes. (#280)
- A long paste into `/` no longer freezes merl and then aborts it: the query holds up to 1,000
  characters, and a paste goes into a prompt or a list's query in one go, searched once. It went
  in one key at a time, compiling the query again at every character, and a query past the regex
  size limit aborted merl with the terminal left in raw mode. (#267)
- Scrolled into a line that wraps, `t`, the tree shown again or a resized terminal leave the
  cursor on its own line. It was drawn one row below the line the status bar names, and a letter
  typed after Enter landed on the line above the highlighted one. (#412)
- A named pipe (FIFO), a socket or a device in the project is not in the tree and not searched,
  so `u` and `D` no longer freeze merl and `s` no longer stalls on one, and `merl PIPE` exits
  with `not a regular file` instead of hanging before its first frame. (#405)
- A symbolic link to a directory is a directory in the tree, read from disk when you expand it,
  like an ignored one; its files open, read-only when they lie outside the project. The walk does
  not follow it, so the searches read what they did and a link to `..` pulls nothing in. It was
  a file row that said `Is a directory`. `merl --review` opens on the first file with a hunk, and
  `c` and `C` pass the link as they pass a submodule; a link first in the branch made merl exit.
  Ctrl+N refuses a path a link leads out of the project with `outside the project`; it created
  the file out there. (#404)
- Inside tmux with its default settings, Ctrl+C and Ctrl+X copy: the text goes to a tmux paste
  buffer and, on tmux 3.2 and newer, to the terminal's clipboard. tmux's default `set-clipboard
  external` dropped the copy, and the status said `copied` while nothing was copied. (#395)
- In SQL, `d` on a schema goes to its `CREATE SCHEMA`: `CREATE TABLE shop.tariffs` declares
  `tariffs`, and no longer counts as a declaration of `shop`. `d` on `public` in `public.orders`
  jumped to `CREATE TABLE public.orders`, and on a schema created once offered every object in it.
  (#471)
- In a shell script, `d` on a name a `local` (or a `declare` / `typeset` without `-g`) binds in
  the function around the cursor lands on that local, and a local of another function is no
  longer offered: it opened a picker of every function's local and the function of that name,
  and jumped to another function's local when it was the only match. (#470)
- In SQL, a common table expression is a declaration only inside its own statement, where it
  wins over a table of its name: `d` on the table in the CTE's own body, or anywhere else in the
  project, goes to the `CREATE TABLE`, and after the `AS (…)` to the CTE, instead of a picker of
  both. (#472)
- In Zig, `d` on a local or a parameter lands on it, `(local)`: a `const` or `var` inside a
  function is no longer offered anywhere outside that function, and a declaration without `pub`
  no longer from another file, so `cap` in one function stopped offering, or jumping to, the
  `cap` of another. (#469)
- The lesson panel of `merl --tutor` and `merl --drill` grows to its text wrapped at the pane's
  width, and the code above gets the rows that are left. At 80 columns six lessons were cut after
  two rows, in lesson 3 before the key it asks you to press. (#261)
- The Linux binaries run on glibc 2.17 and newer: the x86_64 one needed 2.39 and stopped at start
  on Ubuntu 22.04, Debian 12 and older, the aarch64 one 2.18. A release that would need more
  now fails before the Homebrew tap moves to it. (#394)
- `d` in Python no longer reads the words of a comment after a plain `import a, b  # c, d` as
  imports: `d` on a name that follows a comma there went through a made-up import. (#298)
- `d` in Go reads a raw string ending in a backslash, `` `\` `` or `` `C:\` ``, as ending at its
  backtick. It kept the string open, and every declaration after it in the file answered
  `no definition`. (#325)
- `d` in shell scripts, Makefiles, Dockerfiles, YAML, SQL and Terraform: a glob such as
  `rm -rf build/*` or a lone backtick in a comment no longer hides the rest of the file, where
  every declaration answered `no definition`. Each reads its own comments, and the shell, a
  Dockerfile and Terraform their heredocs. (#436)
- A symbolic link to a file that lies outside the project opens read-only, `outside the
  project`, as a file behind a directory link out does. It opened editable, and a save wrote the
  file out there. A link to a file inside the project stays editable. (#448)
- In `merl --review`, Enter on the panel row of a symbolic link to a directory opens nothing
  and leaves the file shown and the status bar as they were. It put the raw OS error with the
  whole path in the status bar, `…/alink: Is a directory (os error 21)`. (#449)
- `d` in Go on the blank identifier `_` answers `no definition for _` at once. It jumped to an
  earlier `_`, as if it were a local of that name; every `_` is a fresh discard. (#476)
- `d` in a Makefile no longer reads an assignment inside a recipe, `GO=$(GO) ./build.sh`, as a
  declaration of the variable: a recipe line is a shell command, and `d` on `$(GO)` jumps to the
  `GO ?= go` make knows instead of offering both. A tab-indented assignment inside an `ifeq`
  outside any rule still declares. (#477)
- `d` in PHP reads `#` in PHP code as a comment, as `//` is, save `#[`, which opens an
  attribute: a glob such as `# loads lib/*` no longer hides the rest of the file, where every
  declaration answered `no definition`. The `#` of the HTML, CSS or JS around `<?php … ?>`
  stays text. (#488)
- `d` in C# reads a verbatim string ending in a backslash, `@"C:\"`, as ending at its second
  `"`, and `@$"…"` as the verbatim string it is. It kept the string open, and every declaration
  after it in the file answered `no definition`. (#475)
- `d` in Rust reads a string that runs over several lines as a string, a raw `r#"…"#` included:
  a declaration-shaped line of a test fixture or a `--help` text inside one is no declaration,
  and `d` on a word inside a string says `no definition` at once, save on the `{name}` a format
  string captures. A word of prose was looked up as a name: `Choose` in an error message jumped
  to a `struct Choose`, and `to` or `with` searched every dependency for a picker of namesakes.
  A lifetime, `'a`, opens no string. (#346)
- `d` in Ruby reads `#` comments, heredocs (`<<~SQL`, `<<-'EOS'`), `=begin` blocks and what
  follows `__END__` as Ruby writes them: the SQL of a migration's heredoc and the old code of a
  `=begin` block declare nothing, and a comment holding an odd number of backticks no longer
  hides every declaration below it in the file. Ruby was read with the C family's `//`, `/* */`
  and backtick template, so `User#prepare!` in mastodon answered `no definition`. (#379)
- `d` in C++ reads a raw string, `R"( … )"`, `R"sql( … )sql"` or `u8R"( … )"`, as a string: a
  declaration-shaped line inside one, such as a banner holding `struct Basket {`, is no longer
  offered beside the real declaration. (#465)
- A file that will not open is named in the status bar as an open file is, from the project root,
  with the reason in a few words: `src/locked.txt: permission denied`. It was the absolute path
  and the OS text, `Permission denied (os error 13)`, and the path could push the reason off the
  line; a path still too long for the pane is cut from the left. (#403)
- `d` in Elixir reaches the dependencies in `deps/`, which `mix new` gitignores: `Jason.encode!`
  jumps to `deps/jason/lib/jason.ex`, read-only, where it said `no definition` or landed on a
  namesake of the project. A module's qualifier narrows the search to the dependency that
  declares the module, so `Phoenix.LiveView.assign` finds `phoenix_live_view`, not Plug's
  `assign`. (#437)
- `d` in C and C++ on a type name lands on its class or struct. It offered a picker of the
  class, every `class X;` forward declaration in other headers and every constructor, and C's
  `typedef struct X { … } X;` as two rows; a bare `X` now lands on `} X;`, `struct X` on the
  opening line. With the class declared once, `Status::Corruption(…)` resolves `via Status`
  again instead of offering every `Corruption`, and `struct DBImpl::Writer {` is found as
  `Writer`, in `d` and `D`. A forward declaration inside a class body, two classes of one name
  and a construction, `Status(…)`, keep their pickers. (#368)
- `d` in C and C++ on a function jumps to its definition instead of offering it beside its
  prototype, `add: by name, 1 definition, 1 prototype`, and on a global past its `extern`
  declaration; overloads and `#if` / `#else` variants keep their picker. A `static`, a `#define` or
  an unnamed `namespace` of another source file is no longer offered, and a `static` of the file
  on screen is its answer. A `#define X` under `#ifndef X` yields to any other declaration of
  `X`: `strcasecmp` in redis jumps to the system's instead of a Windows-only header. (#364)
- `d` in TypeScript and JavaScript on a name imported from a package that is not installed
  (a fresh clone, a package of a monorepo not bootstrapped) lands on its import line and says
  `via import mobx-react (not installed)`. It offered the project's namesakes as if one of them
  were the answer, or jumped to the only one. (#392)
- `d` in TypeScript and JavaScript on a name of a destructuring or a parameter list wrapped one
  name per line, as prettier writes them, lands on the line of the name, on the name, instead
  of the `const {` or `function Row({` above it, so a second `d` goes on from there. (#393)
- `d` in Rust on `x.name` with no `()` behind it lands on the field `name: T` of a struct, one
  row per struct, and when the project has none, on the `pub` fields of the standard library and
  the dependencies; it landed on a method or a local of the name. A word inside an attribute is
  the macro it names (`#[derive(Debug)]`, `#[test]`, `#[tokio::main]`) or declared nowhere
  (`#[cfg(test)]`, `#[allow(…)]`), never a project item called the same. `Mode::Auto`, and a
  bare `Auto` behind a `use Mode::*;`, land on the enum variant, which said `no definition` or
  landed on a struct of its name. (#370)
- `d` in a Makefile finds a variable set only by `CFLAGS += -Wall` or for one target,
  `release: VERSION := 1.0`, where it answered `no definition`. A plain `CFLAGS = -O2` stays
  the only answer where there is one. (#499)
- `d` on a named argument, an object literal's key or a JSX attribute no longer lands on
  whatever else is spelled so: `vm.followTopic(followedTopicId = "a")` in Kotlin jumped to another
  file's local `followedTopicId`, `context.report({ node: lastItem })` to the enclosing function's
  parameter `node`, `getMany(ids: …)` in PHP listed fifteen `$ids =` lines. What it names is looked
  for in the callee or the literal's type only; when that is outside the project or not found, the
  status line says `node: key` or `ids: argument label` and nothing opens. Python, TypeScript,
  JavaScript, Kotlin, Swift, C#, PHP and Ruby. (#315)
- `d` in C and C++ on `x->name` or `x.name` lands on the field `name` of a struct, union or
  class, where it landed on a function, a `#define`, a global or a type of the same name, or
  said `no definition`: in redis `n->data` jumped to a `#define data`, in leveldb `m->level` to
  a method `level()`. Several fields of one name are a picker. A called `x->name(…)` is a method
  or a function-pointer field, never a free function. When the project has no field of the name,
  the system headers are searched for fields only, so `st.st_size` still finds `struct stat`. In
  a C++ constructor's `: filename_(name)` the name lands on the class's own field. (#359)
- `d` in Python on a module's name opens the module at its first line, `repos: module
  app/repos.py`: a word in the path of an import line (`repos` in `from app.repos import
  UserRepo`, `json` in `import json`), and a name an import binds to a module outside the
  project (`serializers` behind `from rest_framework import serializers`, `json` in
  `json.dumps`). It jumped to any method of the name in the dependencies, landed `json` in the
  base interpreter's pip, or said `no definition`. (#333)
- `d` in Python no longer jumps to a namesake that cannot be the answer. `self.client` in a
  subclass of Django's `TestCase` offers only what project subclasses of the class set, and
  says `no definition` without one, where it jumped to any project class's `client`.
  `User.objects` with `User` a class imported from a dependency is a member of `User` there,
  never a module-level `objects` of another package. The one method of a name found outside the
  project is offered rather than jumped to when a field of that name is declared outside too:
  `m.return_value` on a `mock.Mock` jumped to anyio's `TaskHandle.return_value`. (#342)
- `d` in Swift on a type the project declares lands on its `class`, `struct` or `enum`, where it
  listed every `extension` of it beside the type (a picker of 24 for Alamofire's `AFError`). A
  type the project only extends, such as Foundation's `Data`, offers its extensions rather than
  jumping into one as if it were the type. And a `let` or `var` inside a function is no longer a
  candidate behind a `.` or in another function: `session.request` lands on the method, not in a
  picker beside a test's `let request`. (#371)
- `d` in Ruby no longer jumps to a local of another method. A local `name = …` is a candidate
  only in its own method or block, and `@name = …` only for `@name`, in its own class, the class
  reopened in another file included. Behind a dot, as in `x.name`, only a `def`, an `attr_*` or
  an `alias` of the name answers: `uri.scheme` went to some other method's `scheme = …`, and
  `@name = name` read as a declaration of the `name` on its right. (#383)
- `d` in C# no longer lands on a project namesake of something the project does not declare.
  `Task.Delay`, `HttpStatusCode.Created` and any member behind a type name the project declares
  nowhere say `no definition`, where they jumped to a property or method of the same name; a
  private member of another type, and a local or a local function of another method, are no
  longer offered, so `claims.Remove(…)` stays off a private `Remove` of a test mock. (#355)
- `d` in a Makefile finds a variable declared by `define NAME` … `endef`, behind `export` or
  `override`, and `D` lists it. (#468)
- `d` in a Makefile with unsaved edits no longer drops an assignment the edits moved onto a
  line that is a recipe line on disk. (#505)
- `u` in a Makefile marks as declarations the lines `d` counts: not an assignment inside a
  recipe, and a `+=` or target-specific line when nothing assigns the name plainly. (#504)
- `d` in C++ counts the qualifier written on a declaration's line: on `Drawer::Scanner` it lands
  on the body `struct Drawer::Scanner {`, not on the forward declaration in the class, and on
  `Tariff::describe` on the out-of-line `std::string Tariff::describe()`. (#508)
- `d` on a word its line declares elsewhere, the call in `let total = total(order)`, looks it up
  as on any other line instead of offering its namesakes as "at a declaration". (#317)
- Enter in `D` on a large project opens the row the list ranks first, even when pressed before
  the list has caught up with the query, and a name holding `$`, `^`, `!` or `'` is found as
  typed. (#293)
- A new file that cannot be made, and a save that fails, say why in a few words as a file that
  does not open does, without `(os error N)`. (#507)
- `d` in Elixir lands on a function's parameter or a local bound above the cursor, and a bare
  call, an `@spec` or a module attribute on its own module's declaration first. (#460)
- `d` in Lua follows `require` to the module's file and reads `mod.name`, `mod.T.name` and
  `T.name` in the table it names, instead of offering every function of that name. (#462)
- `d` in Go on a parameter used in a body written on its function's own line, `func cut(xs
  []int, n int) []int { return xs[n:] }`, lands on the parameter. It jumped to a package-level
  constant or variable of the same name. (#524)
- `d` in Python on a bare call, `next(steps)`, never offers a method of that name: a `def` in a
  class is reached through a value or the class. Where the file declares a module-level `def
  next`, `d` jumps there instead of offering it beside `Courier.next`. (#522)
- `d` offers what implements a member only on the name its line declares. On another
  occurrence of the word on that line, such as a function its one-line body calls, a parameter
  of the same name or a Go type in its signature, it offered the implementations too; it now
  looks that word up as on any other line. (#517)
- `d` in Swift on the name a `for`, an `if let`, a `guard let` or a closure's parameter declares
  answers as on a `let`: at a declaration, its namesakes elsewhere offered in a picker, and the
  line itself when it has none. It jumped to a namesake elsewhere, often a field of the same
  name. (#525)
- `d` in Ruby on a method's parameter names it by its method in the status line,
  `SessionsController.on_success.user (local)`, as a local of the method is named: it read
  `SessionsController.user`, the way a field would be. (#526)
- The viewed marks of a review, a theme that cannot be read or saved, a review that cannot be
  listed again and the drill's log say why in a few words when a file fails them, without
  `(os error N)`. (#516)
- `d` in TypeScript and JavaScript on a name inside an arrow function written on one line,
  `items.map(item => item * 2)`, lands on the arrow's parameter, `(local)`. It jumped to a
  module-level `const` of the same name. (#531)
- `d` in TypeScript finds a method whose parameter is typed by an inline type literal,
  `paint({ a }: { a: number; b: number }) {`: the `;` inside it read as the end of a call, so
  `d` on `paint` elsewhere said "no definition". (#528)

## [0.7.0] - 2026-09-25

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
  Node loads: the nearest `node_modules` that has the package or its `@types`, and a farther one
  when the nearer lacks the imported path (`lib/extra`), unless the nearer maps its paths with
  `exports`, which is then looked for as before; pnpm's link is followed to the version it
  points at, whatever the store directory is called, and a copy of JavaScript alone is read
  with the nearest declarations further up. A workspace root's
  copy, a copy another package depends on and another version in pnpm's store are no longer
  offered beside it: a name only such a copy declares is found `by name`, no longer
  `via import`, and one the copy's entry exports under another name
  (`export { parseCookie as parse }`) is followed to its declaration. A module of Node's own, such as `buffer`, is looked for as
  before, whatever npm polyfill of that name is installed, and a `node:` import is never a
  project file: `node:util` is `@types/node`'s, not a `src/util.ts` under `"baseUrl": "src"`. A
  workspace package linked into `node_modules`, and an alias import such as `@/lib`, is the
  project's own: `d` finds the name in its source, `by name`, and no longer in an old published
  copy another package depends on; a name it only hands on from a dependency is found outside,
  `by name`. (#141)
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
- `?` and the README key table list the keys under what they do: files and jumps, search,
  review, editing, selection, movement, panels, general, instead of one flat list. (#188)
- Opening another file no longer empties the undo history: each file keeps its own for the whole
  session, as a VS Code tab that stays open does. Fix a line, `d` to see what it calls, `[` back,
  and Ctrl+Z still takes the fix back; Ctrl+Y works the same. A file an agent wrote while you
  were in another one comes back with the write as one more step on top, as if it had been on
  screen. (#163)
- Search ignores case whatever letters the query has: `/`, `s`, `D` and `o` find `SameCancel`
  for `sameCancel`, where one capital used to make the whole query exact and find nothing. There
  is no switch; `u` still lists a word's uses in its exact case. (#174)

### Fixed

- `d` on a default import in the project lands on its module's `export default`, no longer on
  what the module exports under that name otherwise (`export { x as dlocal }`). (#141)
- Opening `/` again on a pattern that matches nothing in the file says `no match`, as typing it
  did, instead of `0/0`. (#174)
- `merl --review=feat` reads what the merge request shows. A local `feat` left from an earlier
  review was opened as it was, so a second review showed the old code, and the base stayed where
  this clone last fetched it, so what the branch took in from `main` looked like its own. Now the
  branch and the base are fetched together and the local branch is brought to what was pushed,
  after a force-push too. Your own commits are never rewritten: when the branch has diverged, or
  local changes are in the way, the status bar says `diverged from origin/feat` or `behind
  origin/feat`, and `origin/feat not fetched` when the fetch failed. (#181)
- A file named with `[ ]`, `*` or `?`, such as `app/[id].tsx`, shows its own changes in the
  gutter and in review; git read the name as a pattern, so `app/d.tsx`'s marks, ghosts and
  hunks showed up in it. (#221)
- With `diff.interHunkContext` in the git config, two edits a few lines apart are two changes
  again: the gutter no longer marks the unchanged lines between them, `c` stops on both, and
  the second one's deleted line is shown. (#220)
- A row that starts with `⚠️`, `✔️` or `➡️` is drawn in place after Ctrl+D and the other
  scrolls, not a cell late. (#206)
- `d` on a field rebound from itself, `this.submit = this.submit.bind(this)` or
  `self.model = self.model.to(device)`, answers instead of aborting merl on a stack overflow
  with the terminal left in raw mode. (#175)
- Tab over a selection of several lines indents each of them, in one undo step, instead of
  replacing them with one indent. (#176)
- `d` on a name inside a call wrapped onto the next lines (`cast(`, `make(`, `new(`) no longer
  aborts merl. (#178)
- Under `--tutor` and `--drill` the `?` list and the pickers stop above the task panel, so the
  task and its tick stay in sight. (#222)
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

[Unreleased]: https://github.com/Maksim-Burtsev/merl/compare/v0.8.3...HEAD
[0.8.3]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.8.3
[0.8.2]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.8.2
[0.8.1]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.8.1
[0.8.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.8.0
[0.7.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.7.0
[0.6.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.6.0
[0.5.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.5.0
[0.4.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.4.0
[0.3.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.3.0
[0.2.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.2.0
[0.1.1]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.1.1
[0.1.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.1.0
