# Changing `f`

`f` (`src/app/collapse.rs`) folds by the rules decided in #598, in the languages that pass the
fold bench and nowhere else: Python, the brace languages (`src/app/collapse/braces.rs`), Ruby,
Lua, shell, zsh and RBS (`src/app/collapse/words.rs`), Elixir (`src/app/collapse/elixir.rs`), Erlang
(`src/app/collapse/erlang.rs`), Julia (`src/app/collapse/julia.rs`), Nix (`src/app/collapse/nix.rs`),
Make and CMake (`src/app/collapse/make.rs`), Starlark (`src/app/collapse/starlark.rs`), Clojure,
Common Lisp, Racket and Scheme (`src/app/collapse/lisp.rs`), YAML, JSON, TOML, Terraform, HTML,
Vue, Svelte, Astro, XML, CSS / SCSS and Markdown (`src/app/collapse/spans.rs`) and SQL
(`src/app/collapse/sql.rs`) today, the rest in epic #623, each saying `no fold rules for .EXT`
until then.

- **The bench**, `tools/fold-bench/run [--lang go,rust]`: `f` pressed on every line that starts
  a fold in real projects, compared with the reference: Python's own `ast` for Python, and for
  the rest what Neovim folds with nvim-treesitter's `folds.scm` in the projects of
  `tools/d-bench/projects.tsv`, recorded once into `tools/fold-bench/folds/LANG.tsv`
  (`record.py`, never in CI). It prints per language the exact folds, wrong ranges, nothing
  folded and p50 ms. A language still off is pressed with the rules forced on and marked `off`:
  that row is where its work starts, and it never fails the run.
- **A language turns `f` on** at 99 % exact in the bench, with a fold fixture in `tests/folds/`
  and its bench row; the PR commits the new `tools/fold-bench/baseline.tsv`
  (`--update-baseline`).
- **A change to `f`** keeps `tests/folds/` green and no bench line of a language that is on worse
  than `baseline.tsv`: the run exits 1 on any.
