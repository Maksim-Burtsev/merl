# `d` bench — the prototype of 2026-09-28

A throwaway that measured how often `d` lands where a language server would, in one real project
per language. It is the starting point of the bench issue in the tests epic, not a tool to keep
as it is: nothing here is wired into `cargo test`, CI or the release.

## What ran

1. `bench.py sample LANG ROOT N OUT.tsv` picks identifier cursors outside comments and strings,
   mixed by shape: `member` (after `.` or `->`), `path` (after `::`), `call` (a bare name before
   `(`), `type` (capitalised), `name`. Seeded, so a rerun picks the same cursors.
2. `zz_bench.rs` goes into `src/app/tests/` with `mod zz_bench;` in `src/app/tests/mod.rs` (never
   committed). `BENCH_IN=cur.tsv BENCH_OUT=merl.tsv cargo test --release zz_bench -- --ignored`
   presses `d` on every cursor, one `App` per project, and writes the outcome (`jump`, `picker`,
   `stay`), the status line, every target (`path:line`) and the milliseconds.
3. `bench.py oracle LANG ROOT cur.tsv orc.tsv` asks a language server the same question over
   LSP (`textDocument/definition`, plus `declaration` for C and C++). `run-oracles.sh` starts one
   per language. The servers were installed into a scratch directory, never into merl:
   pyright, typescript-language-server with `typescript@6` (npm's `typescript@7` has no
   `tsserver.js`), gopls, rust-analyzer, clangd and sourcekit-lsp from the Xcode tools,
   intelephense. Java, Kotlin, C# and Ruby had no server on the machine; an agent judged their
   cursors by reading the code.
4. `bench.py score cur.tsv orc.tsv merl.tsv score.tsv` gives each cursor a verdict: `ok` (the
   jump is the oracle's target, ±1 line), `pick-hit` (a picker holding it), `WRONG` (a jump
   elsewhere), `pick-miss`, `none`, and the unscored `on-decl` / `oracle-none/*`.

`data/` holds the cursors (`cur-*`), the oracle answers (`orc-*`) and the verdicts (`score-*`),
with the project roots as `proj/NAME`. The projects were shallow clones of the default branch on
2026-09-28: paperless-ngx (Python, `uv sync`), outline (TypeScript, no `node_modules`), eslint
(JavaScript, `npm install --ignore-scripts`), caddy (Go, `go mod download`), ripgrep (Rust,
`cargo fetch`), redis (C, a generated `compile_commands.json` for `src/*.c`), leveldb (C++, CMake),
koel (PHP, no `vendor/`), Alamofire (Swift), halo (Java), nowinandroid (Kotlin), eShop (C#),
mastodon (Ruby).

## What it found

| Language (project) | direct hit | picker with the answer | wrong jump | miss |
|---|---|---|---|---|
| Python (paperless-ngx) | 70% | 12% | 1% | 17% |
| TypeScript (outline, project code only) | 72% | 10% | 5% | 13% |
| Go (caddy) | 62% | 12% | 4% | 22% |
| Kotlin (nowinandroid, judged) | 57% | 27% | 7% | 10% |
| Ruby (mastodon, judged) | 52% | 18% | 6% | 24% |
| JavaScript (eslint) | 46% | 34% | 5% | 15% |
| C# (eShop, judged) | 37% | 52% | 0% | 11% |
| PHP (koel, no `vendor/`) | 35% | 45% | 0% | 20% |
| Rust (ripgrep) | 34% | 33% | 9% | 23% |
| Java (halo, judged) | 30% | 38% | 0% | 32% |
| C (redis) | 28% | 17% | 5% | 49% |
| Swift (Alamofire) | 28% | 59% | 4% | 9% |
| C++ (leveldb) | 19% | 43% | 3% | 35% |

The judged rows count only targets inside the project (27–37 cursors each). A separate probe of
`x.member` jumps found 31 of 45 wrong in Ruby and 8 of 35 in C#. Jumps proven `via import` or
`via type` were right almost always (Python 127/127, Go 66/66, TypeScript 52/54); the wrong jumps
come from `by name, 1 match`.

`reports/` has one cause analysis per language: every wrong jump, and samples of the misses and
pickers read by hand, grouped by the rule that is missing, with a minimal repro, a fix sketch
inside "rules over lines", its risk and size. `AUDIT-2026-09-28.md` is the audit of the project
done the same day.

## Pitfalls met

- zsh does not split `$var` into words: `run-oracles.sh` is bash.
- rust-analyzer sometimes exits under load; `oracle` resumes from the answers already written.
- Without the project's dependencies installed, the oracle cannot answer calls into them, and
  its answers for the rest need a human pass: up to 60% of its disagreements with merl in
  TypeScript and PHP were the oracle's (shorthand properties, contextual types, uninstalled
  packages).
- Timings mean something only with `vm.loadavg` under ~8.
