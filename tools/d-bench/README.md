# `d` bench

How often `d` lands where a language server would, per language, in 36 real projects pinned to a
commit: 7,997 cursors, each with an answer recorded once and reviewed, and the table master
scores on them (`baseline.md`). A `d` change runs it and shows no language worse than master
(`AGENTS.md`, `## Changing d`).

```sh
tools/d-bench/run                    # every language: 5.4 min on a warm cache (2026-09-28)
tools/d-bench/run --lang go,rust     # one language: 40 s for Go
tools/d-bench/run --project caddy
tools/d-bench/run --update-baseline  # a PR that changes the table commits the new baseline
```

The runner clones each project into the cache (`--cache`, `$D_BENCH_CACHE`, default
`~/.cache/merl-d-bench`) at its pinned commit and runs its install command until it succeeds
once (marked in the clone's `.git`); a clone already at that commit and installed is only read. It then plays every cursor through the `#[ignore]` test
`src/app/tests/d_bench.rs` in release (one `App` per project: jump to the line, set the column,
converted from code points to merl's bytes, press `d`), prints the table and under it every wrong
jump on a row it does not score, and diffs every cursor against `baseline.tsv`: each cursor that
got worse, new wrong jumps first (a new one on a row not scored among them), and exit 1 when there
is any (a fixed cursor does not pay for a broken one). `--selftest` checks the scoring, this
gate and `record.py`'s tokenizer on made-up rows; CI runs it. `last-score.tsv` in the cache has every cursor's verdict, status line, merl's
targets and the answer; `--merl <cache>/last-merl.tsv` scores the last run again without
replaying it.

Times mean something only with `sysctl -n vm.loadavg` under ~8; the baseline notes its load.

## Files

| file | what |
|---|---|
| `projects.tsv` | name, language, git URL, pinned commit, install command (`uv sync`, `npm install --ignore-scripts`, `pnpm install --ignore-scripts`, `go mod download`, `cargo fetch`, `dart pub get`, `composer install`, `-`) |
| `cursors/LANG.tsv` | `id project file line col shape word`: identifiers outside comments and strings, 1-based line, 0-based column in code points; `shape` is `member` (after `.`, `?.`, `->`), `path` (after `::`), `call`, `type` (capitalised) or `name` |
| `answers/LANG.tsv` | `id targets skip note`: the definition as `path:line`, project-relative, `~/` or absolute outside the project (a dependency or the toolchain's standard library); `skip` holds the reason a debatable answer is not scored |
| `baseline.tsv`, `baseline.md` | master's verdict per cursor, and its table |
| `run` | the runner |
| `record.py`, `install-servers.sh` | the recording, once, on a recording machine |

## Verdicts

A jump is right when it lands within one line of the answer; for an answer outside the project,
the same file name under the same directory counts (a dependency's copy in another cache). pnpm
keeps a copy of a package per set of peers it resolved with (`.pnpm/astro@7.2.10_<peers>/`), the
same files in each: a file in one copy counts as the same file in another copy of that version.

| verdict | merl |
|---|---|
| `ok` (direct hit) | jumped to the answer |
| `pick-hit` (picker with the answer) | offered a picker that holds it |
| `WRONG` (wrong jump) | jumped elsewhere |
| `pick-miss`, `none` (miss) | a picker without it, or "no definition" |
| not scored | `skip` (the answer is debatable), `no-answer` (the oracle found nothing, or the definition is outside the project for a judged language), `on-decl` (the cursor is on the definition) |
| `skip-WRONG`, `on-decl-WRONG` (not scored, jumps elsewhere) | jumped from a row not scored, away from its recorded answer: not within a line of it, nor into its file outside the project (an SDK's generated interface) |
| `no-answer-jump`, `skip-jump` (not scored, jumps elsewhere) | jumped from a row not scored that has no answer recorded: nothing to check the jump against |

The rows not scored are where most wrong jumps hid (#1092). These verdicts stay out of the rates,
are counted in the table's `not scored, jumps elsewhere` column and listed under it, and a new one
fails the gate as a new wrong jump does. A `-jump` row may be right (a builtin found in the
project's own `lib.dom.d.ts`): the list is for reading, the gate for what changed.

## Recording, once

Never in merl, never in CI. The servers go into a scratch directory:

```sh
tools/d-bench/run --project koel                # clone and install the project first
tools/d-bench/install-servers.sh                # pyright, typescript-language-server with typescript@6,
                                                # intelephense, the Vue, Svelte and Astro servers,
                                                # @nomicfoundation/solidity-language-server,
                                                # bash-language-server, gopls, rust-analyzer, starpls,
                                                # lua-language-server, LanguageServer.jl,
                                                # the Dart SDK, a static PHP and composer
tools/d-bench/record.py sample php 300          # -> cursors/php.tsv (seeded: the same cursors again)
tools/d-bench/record.py oracle php              # -> answers/php.tsv, resumes where it stopped
```

`oracle` asks `textDocument/definition` (plus `declaration` for C and C++). clangd and
sourcekit-lsp come from the Xcode command-line tools. clangd needs a `compile_commands.json`: for
redis one was written for `src/*.c` (flags `-std=gnu11 -Isrc -Ideps/hiredis -Ideps/linenoise
-Ideps/lua/src -Ideps/hdr_histogram -Ideps/fpconv -Ideps/xxhash -Ideps/tre -Isrc/modules`), for
leveldb CMake wrote it (`cmake -B build -DCMAKE_EXPORT_COMPILE_COMMANDS=ON`), for SDWebImage
one was written for `SDWebImage/Core/*.m` and `SDWebImage/Private/*.m` (flags `-x objective-c
-fobjc-arc -fmodules -isysroot $(xcrun --show-sdk-path) -ISDWebImage/Core -ISDWebImage/Private
-ISDWebImage/include`), its cursors sampled with `--exclude
Examples,Tests,WebImage,Docs,Scripts,SDWebImageMapKit,include` (`include/` links back into
`Core/`). Solidity's oracle, `@nomicfoundation/solidity-language-server`, reads a project
through its local Hardhat 3, so openzeppelin-contracts installs with `npm ci --ignore-scripts`;
its cursors were sampled with `--exclude test,lib,scripts,fv,certora,docs,hardhat,audits,mocks`
(the library's own `contracts/`), and the ones in inline assembly, Yul builtins, have no answer.
Java, Kotlin, C#
and Ruby had no server on the recording machine: an agent judged their cursors by reading the
code, and a definition outside the project (the JDK, a gem) is `no-answer` there. Groovy was
judged the same way, in two rows: `groovy` (nextflow, Groovy with Java and a Gradle build) and
`jenkins` (pipeline-library, a Jenkins shared library: `vars/` steps and the tests that load them
with `loadScript`, whose `script.call()` is judged to land on the loaded step). Their keys of a
map built at runtime (`config.deployFolder`), Spock labels and `where:` variables are `skip`.

Julia came from a julialang-s3 tarball, `JULIA_DEPOT_PATH` pointing at a scratch depot for both
DataFrames.jl's install and `install-servers.sh`. LanguageServer.jl's analysis process drops
`JULIA_DEPOT_PATH` and reads `~/.julia`, so `oracle` starts it with `HOME` at
`<servers>/julia-home`, whose `.julia` links to that depot. It answers nothing for names from a
dependency (Tables, InvertedIndices) and for some of Base's core types: those rows are `no-answer`.
For a Base generic that DataFrames extends (`parent`, `view`, `copy`, `filter!`) it lists only
the methods outside the project: where the argument is a DataFrames type, the row is `skip`.

Nix (nix-darwin, 2026-10-02) was judged the same way: `nil` and `nixd` both need `nix` itself,
`nil` already to build. A name a function argument, a `let` or an `inherit (lib)` binds answers
with that binding; `cfg.enable` with the option's `mkOption` line (`cfg = config.services.x`); a
path literal (sampled as shape `path`) with its file, or a directory's `default.nix`. A name from
nixpkgs (`lib.mkIf`, `types.str`, `with lib;`), or from `builtins`, is outside the project and
`no-answer`; an option namespace (`config.system`) or an attribute several modules set
(`environment.variables.X`) is `skip`. A lambda argument used on its own line scores as `on-decl`.

CSS (#590) samples Bootstrap's `scss/` (`$variable` uses, `@include` mixins, calls of the
functions it declares, `@import` paths) and the classes of `site/`'s Astro templates
(`--exclude tests,vendor`). `vscode-css-language-server` answers within the open stylesheet
only, so a name used in another file was judged instead: its top-level declarations in `scss/`,
or for a path the file Sass loads (`judged` in the note). The classes were judged by reading the
stylesheets: the top-level rule of `scss/`, or of the example's own stylesheet; a class the
utilities API or a Sass loop generates, a state class such as `active` and one styled only inside
other components are `skip`.

Vue (gitea), Svelte (immich) and Astro (starlight) components are sampled from their code only:
the `<script>` blocks and an Astro frontmatter, the template's expressions (`{{ }}` and bound
attributes in Vue, `{ }` in Svelte and Astro) and the component names its tags use. Their oracles
are `@vue/language-server@2` with `hybridMode: false` (version 3 answers TypeScript only through
an editor's tsserver), `svelte-language-server` and `@astrojs/language-server`. immich installs
the web app alone and runs `svelte-kit sync` for its `$lib` alias; its `@immich/sdk` is not built,
so the 39 cursors on the API's types are `no-answer`.

After recording, review every cursor where merl and the oracle disagree (`WRONG`, `pick-miss`,
`none` in `last-score.tsv`) and mark the oracle's debatable answers `skip` with the reason: a
shorthand property, a contextual type, a package that is not installed, a declaration the
compile commands do not reach. In the prototype up to 60% of the disagreements in TypeScript and
PHP were the oracle's.

## Review, 2026-09-28

Every cursor where master and the oracle disagreed was read once against the code: 532 in the
nine languages with a server, of which 54 are now `skip` (PHP 15, TypeScript 16, JavaScript 10,
Rust 7, C++ 4, C 2) and 19 got a corrected or completed target (`reviewed:` in the note: a C
definition next to its header declaration, a module's file next to its `mod` line, a
TokenStore method next to its type). Go, Python and Swift needed nothing. The agreements (`ok`,
`pick-hit`) were not re-read. The four judged languages were judged cursor by cursor already;
their in-project targets were checked line by line when they were written down.

The components, recorded on 2026-10-02 (#595), were reviewed the same way: 143 disagreements,
of which 19 are now `skip` (12 on `@immich/sdk`, 4 object-literal keys, 2 custom element tags, 1
attribute name) and 4 got lodash-es's code next to its `@types/lodash` declaration. An answer in
the oracle's own TypeScript (`lib.dom.d.ts`) is `skip` as the server's own stub, 151 of them.
The rest stand: `d` on a Vue import picks among the copies in `vue/dist` where the oracle
follows the re-export into `@vue/reactivity`, and SvelteKit's `$app/…` modules stop at their
import line.
## Starlark, 2026-10-03

rules_go's 270 cursors were recorded with starpls 0.1.22 and no Bazel on the machine: starpls
answers inside the workspace and fetches no external repository, so a name a `load` takes from
`@bazel_skylib` or another repository is `no-answer`, as is a builtin (`ctx.actions`,
`attr.label`): 166 cursors, and 20 more stand on their declaration. Of the three misses, all are keyword arguments, which
starpls takes to the callee's parameter and `d` refuses on purpose (#431).

## Lisp, 2026-10-06

Five rows, one per dialect merl reads (#428). Their cursors skip keywords (`:key`, `#:key`) and
the name a defining form introduces; a word right after `(` is a `call`, after `ns/` or `pkg:` a
`path`, and the rest mostly `name`.

- **clojure** (babashka): clojure-lsp, with no `clojure` or `lein` on the machine to compute a
  classpath, so `oracle` hands it one, every `:paths` and `:extra-paths` directory of `deps.edn`
  that exists. The submodules (sci, babashka.fs, ...) are not cloned and `test-resources/`, the
  tests of third-party libraries, is excluded from the sample (`--exclude test-resources`): what
  comes from them, and `clojure.core`, which clojure-lsp answers inside a jar, is `no-answer`.
- **racket** (drracket): racket-langserver from minimal-racket, which answers only once a file has
  expanded (`oracle` waits for the file's diagnostics). drracket's own collections are not
  installed, so the clone's packages go on `PLTCOLLECTS` after the defaults, and the packages
  drracket-core-lib needs and minimal-racket lacks (images-lib, typed-racket-more,
  macro-debugger, ...) went into a scratch `PLTADDONDIR` with `raco pkg install --scope user`. A
  name the installed drracket-tool-lib also has is answered in that copy, matched by its path.
- **elisp** (magit), **scheme** (chibi-scheme), **commonlisp** (lem): no server; an agent judged
  80 cursors each by reading the code, as for Java. A dependency (transient, compat, Quicklisp
  libraries), the builtins and a primitive chibi implements only in C are `no-answer`; a generic
  function's answer is its `defgeneric` and every method. `sample` reads files with `newline=""`
  since magit-diff.el holds a lone CR, which Python would count as a line break and merl does not.

## Bash and Lua, 2026-10-18

Two projects each (#1091); the row adds them up. `record.py sample bash 55` and `sample lua 55`
draw the same cursors again: 56 in bats-core, 50 in acme.sh (whose `source` lines name only
variables, so no `path` cursor) and 55 in each Lua project.

- **bash** (bats-core, acme.sh): bats-core's libraries, `.bats` tests and extensionless `libexec/`
  scripts (found by their `#!` line), and acme.sh's main script with the `dnsapi/`, `deploy/` and
  `notify/` plugins it sources. `sample` keeps `$name` inside double quotes, skips builtins and
  common commands, and takes the last segment of a `source` path as a `path` cursor; three more on
  the `source` lines of bats-core's `.bash` files were added by hand. bash-language-server 5.8.1,
  default settings, answers within a file and the files it sources by a static path. It analyzes a
  file only if it opens after the server has initialized, so `oracle` opens none before it settles.
  Where it found nothing, 33 answers were read from the code (`judged` in the note), 18 of them a
  plugin's call into acme.sh, which sources the plugin. A variable set outside the project
  (`OSTYPE`, `ME_Secret`) is `no-answer`. `skip`: 4 sampler misses (a `case` label, words inside a
  multi-line string), a formatter function each formatter script defines (which one runs is a
  runtime choice), an executable on `PATH` and a variable's name used as an array element.
- **lua** (nvim-treesitter, kong): nvim-treesitter's `lua/`, `plugin/` and `scripts/`, and kong's
  `kong/`. lua-language-server 3.19.2, LuaJIT with `lua/?.lua` on the path. An answer in its own
  stubs (`type`, `assert`, `require`) is `skip`, as `lib.dom.d.ts` is for TypeScript: 19 rows.
  Where it adds its stub to a file's own `local type = type`, only the local stays (8 rows,
  `reviewed:`). Two answers into kong's `spec/` mocks are `skip`. Its 26 empty answers, mostly
  members of Neovim's `vim` and OpenResty's `ngx` (no stubs for them here) and fields of tables
  built at runtime, are `no-answer`.

## More rows, 2026-10-19

The `d` pass of 2026-10-18 drew cursors where the bench had no row or few scored ones (#1091).
They join the rows below at the projects' pinned commits, with four projects added (dio, lemmy,
uptime-kuma, and gitea for Go) and koel's `vendor/` installed, and every answer was read once more
before it went in. Several sets were
picked rather than sampled, to show where merl jumps by name or has to infer a type: they say how
merl does on that code, not on code at large, and they move a row's rate. Swift's first 270
cursors, sampled, score 75 % direct; with the 459 picked ones the row scores 91 %.

- **dart** (dio, `dio/lib` only): `record.py sample dart 115 --exclude
  plugins,dio_test,example_dart,example_flutter_app,example,test,doc,scripts` draws the 104
  sampled cursors again (Dart has no `path` cursor, so 115 asked gives 104); ten inside string
  interpolation, which the sampler masks as strings, were picked by hand (`dart0104`–`dart0113`).
  `dart language-server` from the Dart SDK 3.13.5 answered all 114, 24 in the SDK and 1 in the pub
  cache; the 18 disagreements were read, and the server was right in all. merl reads the SDK's
  `lib/` only when `dart` is on `PATH`, so `run` puts `<servers>/dart-sdk/bin` first on `PATH` and
  `PUB_CACHE` at `<servers>/dart-pub-cache`, for dio's install and for merl, and stops when the
  `dart` row is asked for without it; `install-servers.sh` fetches the SDK. dio installs with `dart --suppress-analytics pub get`, as a plain `dart pub`
  writes `~/.dart-tool`.
- **sql** (lemmy, Postgres), judged by reading: 80 cursors in `migrations/*/{up,down}.sql` and 30
  in `crates/diesel_utils/replaceable_schema/`, drawn by the pass's own sampler (comments and
  strings masked, keywords skipped; 30 % members, 20 % calls, 50 % names), as `record.py`'s skips
  `migrations/`. An answer is the definition in effect where the cursor runs: migrations run in
  their folders' order, a `down.sql` right after its own `up.sql`, the replaceable schema after
  them all. A name answers with the latest `CREATE`, `ADD COLUMN` or `RENAME … TO` before that
  point; a column with its `CREATE TABLE` line (or its item in a `CREATE TABLE … AS SELECT`), its
  `ADD COLUMN` or its last `RENAME COLUMN`; `NEW` and `OLD` with the table the trigger is on. A
  view's, a CTE's or a derived table's column, a field of a generic `record` parameter and a name
  a `RENAME TO` gives are `skip` (10); a builtin (`lower`, `now`) is `no-answer` (7). The pass had
  proposed the 47 column answers by name alone, from any table: read against their tables, 28
  moved (`reviewed:`).
- **java** (halo), **kotlin** (nowinandroid), **csharp** (eShop), judged as their first 80 rows
  were: 71, 71 and 68 sampled, and picked ones besides. Java's are 40 getter chains
  (`post.getSpec().getCategories()`, answered with the field Lombok's `@Data` reads) and 12 calls
  with a type witness (`IndexSpecs.<Post, String>single(...)`); C#'s are 55 member accesses on
  locals, fields and parameters, record properties and chains continued on the next line, and 12
  cursors in `.razor` components, which merl has no rules for yet (#1088): misses until it does.
  A string resource answers with its `strings.xml` entry; a protobuf or gRPC accessor is `skip`,
  its `.proto` field kept as the target.
- **swift** (Alamofire), picked: 350 member accesses in `Source/` where merl jumped "by name, 1
  match" and 109 implicit members (`.success(…)`, `.allow`), answered by sourcekit-lsp. The 165
  in an SDK interface it generated are `skip`, as before, with no target kept; merl jumped from 112
  of them then, and the jumps left are its `skip-jump` rows (#1092). Six implicit members in `Tests/` it left unanswered were read from
  the code.
- **ruby** (mastodon), judged: 118 picked partly for Rails idioms, among them ten instance
  variables twice, the cursor on the `@` and on the name (merl finds no word on the `@`: the
  row's misses), and 121 bare calls where merl jumped by name. 27 of those land in the project;
  the rest call a gem, Ruby's core or the keyword `next` and are `no-answer` or `skip`. As before,
  a Rails macro's line (`has_many`, `delegate`, `scope`) answers the method it makes, and a
  column's `db/schema.rb` line is `skip`. The gems are not installed: the gem names in the notes
  were not read from their code.
- **python** (paperless-ngx): 55 cursors picked around Django's dynamic parts (the test client,
  managers, `settings`, mocks, keyword arguments). pyright answered 30; asked again after a 180 s
  warm-up, it answered none of the other 25. Of those, 14 were read from the code (`judged`); a
  manager method, which Django builds from `QuerySet` at runtime, a historical model from
  `apps.get_model` and a field lookup are `no-answer`. In an `APITestCase`, `self.client` is DRF's
  `APIClient`, whose `get`, `post`, `patch` and `delete` override the `Client` methods pyright
  answers with: those rows point at DRF's, three of the first 270 included (`reviewed:`).
- **go** (caddy, gitea): in gitea, 54 sampled and 16 members after an index (`m[k].Get()`); in
  caddy, 14 keys of table-test literals whose type is left out (`[]Token{{Line: 1}}`) and 15
  local names. gopls answered with `GOPROXY=off` and gitea's modules not downloaded, so in gitea it
  knows the project and the standard library only. gitea's `go` line installs nothing; its `vue`
  line stays as it was.
- **js** (eslint, uptime-kuma): uptime-kuma joins, a plain CommonJS Node server with its Vue app's
  helpers, mostly `.js`, beside eslint, whose misses sit in rule modules typed through JSDoc.
  `sample("js", "uptime-kuma", root, 135, exclude)`, excluding `test`, `tests`, `examples`,
  `example`, `benchmarks`, `docs`, `fixtures`, `dist`, `build`, `public` and `push-examples`, draws
  124 (`js0270` on: only two `path` cursors exist, and the draw does not top up).
  typescript-language-server answered, with `npm install --ignore-scripts` done. An answer in its
  own `lib.*.d.ts` is `skip`, as for eslint (11); so are a redbean-node bean's column, which it
  answers with `Bean`'s index signature (3), a destructured `require` it stops at (#328), and
  `require("axios")`, answered with the top of the package's `index.d.ts`. Its 23 empty answers,
  mostly fields of objects built at runtime and knex chains, and three words inside regex literals,
  are `no-answer`.
- **php** (koel): koel now installs `vendor/` (`composer install --no-scripts
  --ignore-platform-reqs`: 196 MB, 201 packages), run by a static PHP 8.4.23 CLI (static-php-cli's
  prebuilt `common` build) and composer 2.10.3, which `install-servers.sh` puts in
  `<servers>/php`. So intelephense answers calls into Laravel, and merl's `vendor/` lookups run.
  The 161 cursors it had answered with nothing were asked again: 131 now land in `vendor/`; the
  rest (`<?php`, words in strings and command signatures, `namespace` lines, Eloquent builder
  chains) stay `no-answer`. `php0300` on are picked: 159 Laravel calls (facades, helpers, Eloquent,
  receivers typed with Illuminate classes) with intelephense's answers, and 21 Eloquent
  `@property` chains read from the code, where intelephense agrees on all 21. A namespace segment
  of a `use` line is `skip`, as before (13 more). With `vendor/` there, two direct hits became
  pickers (`php0073`, `php0258`: a member name with namesakes in `vendor/`), and `d` takes 63 ms at
  p50 instead of 14.

## History

The cursors and answers were recorded on 2026-09-28 by a prototype (branch `proto/d-bench`,
with its per-language cause analyses in `reports/` and the table in #305). The sweep of review
scenarios this bench replaces lives on as the annotated fixtures (`tests/fixtures/README.md`),
which `cargo test` checks.

Pitfalls met: rust-analyzer sometimes exits under load (`oracle` resumes); without a project's
dependencies installed the oracle cannot answer calls into them; `typescript@7` on npm has no
`tsserver.js`.
