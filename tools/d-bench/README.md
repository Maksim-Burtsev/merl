# `d` bench

How often `d` lands where a language server would, per language, in 16 real projects pinned to a
commit: 3,284 cursors, each with an answer recorded once and reviewed, and the table master
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
converted from code points to merl's bytes, press `d`), prints the table, and diffs every cursor
against `baseline.tsv`: each cursor that got worse, new wrong jumps first, and exit 1 when there
is any (a fixed cursor does not pay for a broken one). `--selftest` checks the scoring and this
gate on made-up rows; CI runs it. `last-score.tsv` in the cache has every cursor's verdict, status line, merl's
targets and the answer; `--merl <cache>/last-merl.tsv` scores the last run again without
replaying it.

Times mean something only with `sysctl -n vm.loadavg` under ~8; the baseline notes its load.

## Files

| file | what |
|---|---|
| `projects.tsv` | name, language, git URL, pinned commit, install command (`uv sync`, `npm install --ignore-scripts`, `go mod download`, `cargo fetch`, `-`) |
| `cursors/LANG.tsv` | `id project file line col shape word`: identifiers outside comments and strings, 1-based line, 0-based column in code points; `shape` is `member` (after `.`, `?.`, `->`), `path` (after `::`), `call`, `type` (capitalised) or `name` |
| `answers/LANG.tsv` | `id targets skip note`: the definition as `path:line`, project-relative, `~/` or absolute outside the project (a dependency or the toolchain's standard library); `skip` holds the reason a debatable answer is not scored |
| `baseline.tsv`, `baseline.md` | master's verdict per cursor, and its table |
| `run` | the runner |
| `record.py`, `install-servers.sh` | the recording, once, on a recording machine |

## Verdicts

A jump is right when it lands within one line of the answer; for an answer outside the project,
the same file name under the same directory counts (a dependency's copy in another cache).

| verdict | merl |
|---|---|
| `ok` (direct hit) | jumped to the answer |
| `pick-hit` (picker with the answer) | offered a picker that holds it |
| `WRONG` (wrong jump) | jumped elsewhere |
| `pick-miss`, `none` (miss) | a picker without it, or "no definition" |
| not scored | `skip` (the answer is debatable), `no-answer` (the oracle found nothing, or the definition is outside the project for a judged language), `on-decl` (the cursor is on the definition) |

## Recording, once

Never in merl, never in CI. The servers go into a scratch directory:

```sh
tools/d-bench/run --project koel                # clone and install the project first
tools/d-bench/install-servers.sh                # pyright, typescript-language-server with typescript@6,
                                                # intelephense, gopls, rust-analyzer
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
`Core/`). Java, Kotlin, C#
and Ruby had no server on the recording machine: an agent judged their cursors by reading the
code, and a definition outside the project (the JDK, a gem) is `no-answer` there. Groovy was
judged the same way, in two rows: `groovy` (nextflow, Groovy with Java and a Gradle build) and
`jenkins` (pipeline-library, a Jenkins shared library: `vars/` steps and the tests that load them
with `loadScript`, whose `script.call()` is judged to land on the loaded step). Their keys of a
map built at runtime (`config.deployFolder`), Spock labels and `where:` variables are `skip`.

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

## History

The cursors and answers were recorded on 2026-09-28 by a prototype (branch `proto/d-bench`,
with its per-language cause analyses in `reports/` and the table in #305). The sweep of review
scenarios this bench replaces lives on as the annotated fixtures (`tests/fixtures/README.md`),
which `cargo test` checks.

Pitfalls met: rust-analyzer sometimes exits under load (`oracle` resumes); without a project's
dependencies installed the oracle cannot answer calls into them; `typescript@7` on npm has no
`tsserver.js`.
