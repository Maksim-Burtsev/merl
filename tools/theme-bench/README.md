# Theme bench

How close merl paints a theme to what LazyVim paints with the same colorscheme on the same file:
the share of non-blank characters whose foreground is LazyVim's, per theme and language, and the
chrome colours side by side. It is the yardstick of epic #797; `baseline.tsv` is master's table.

```sh
tools/theme-bench/run                          # every recorded theme, every language: 4.5 min (2026-10-11)
tools/theme-bench/run --theme kanagawa-wave    # its share per language, top mismatches, chrome
tools/theme-bench/run --lang python,go
tools/theme-bench/run --update-baseline        # a PR that changes the table commits the new baseline
tools/theme-bench/run --tree-sitter --lang python   # merl with the hidden `tree_sitter = true` (#802)
tools/theme-bench/run --selftest               # the reading, the scoring, the gate, the stored format
                                               # and themes.tsv covering every shipped theme; CI runs it
```

The runner builds `target/release/merl` (`--merl BIN` runs another build as it is), draws each
file with `--theme THEME` in tmux on a server of its own process, 200 columns wide, with a
temporary `HOME`, reads the screen as `tools/cast.py` does (Pillow) and compares each line, by its
gutter number and its text with blanks dropped, with the recording; a recorded line merl does not
draw counts as all wrong. It prints per language the characters compared and the lowest, median
and highest share over the themes, and exits 1 when a theme × language matches fewer characters
than in `baseline.tsv`, or, on a run of every theme and language, when a baseline row was not
run. A recording drawn from other commits than `tools/d-bench/projects.tsv` pins now is refused:
record it again. With `--theme`, it also prints the theme's most frequent mismatches
(`want #x got #y e.g. token`) and the chrome: merl's background, cursor line, selection, find
match, current find match, line numbers, cursor line number and status line read off its screen
on a probe file (`v`, then a find for `alpha`), LazyVim's from the highlight groups `Normal`,
`CursorLine`, `Visual`, `Search`, `CurSearch`, `LineNr`, `CursorLineNr` and `StatusLine`.

## Tree-sitter

`--tree-sitter` runs merl with `tree_sitter = true` in its config, the colouring of epic #797
(AGENTS.md `## Changing syntax colours`); `baseline.tsv` stays master's, with the setting off, so
the gate still holds every row to it. On 2026-10-12 (#802) Python matched every character in all
92 themes, 19,185 per theme: no miss is left to list. A language that leaves misses lists them
here by cause.

Go and Rust (#805), before injections:

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Go | 14,810 | 99.7 adwaita | 99.9 | 100.0 vscode-light |
| Rust | 17,576 | 90.7 github-dark | 94.4 | 99.9 e-ink-light |

- Injections (#818). Go: the `%` verbs of `Errorf` and `Fatalf` strings, which LazyVim parses as
  `printf`, 38 characters in the themes that colour them apart. Rust: the bodies of macro calls,
  which LazyVim parses again as Rust (`path.display()` inside `format!`, every test of
  `tests/feature.rs` inside `rgtest!`): almost all of Rust's misses, in every theme.
- The theme's own queries (#819): github-dark and github-light append github-nvim-theme's
  `queries/rust/highlights.scm` (attributes, `crate`/`self`/`super`, lifetimes, capitalised
  names), which merl does not load.

Markdown (#803), 18,398 characters per theme: 98.3 % median (97.8 everforest-dark, 99.3
alabaster-light), against 71.3 % with the setting off. Every character of Markdown's own grammars,
`markdown` and `markdown_inline`, and of the Python, Go and Rust its fences hold, matches in the
themes read by cause (tokyonight-moon, dayfox, github-light); the misses are code merl does not
yet colour as LazyVim does, drawn through syntect as before or waiting for Rust's injections:

| cause | tokyonight-moon | dayfox | github-light | lands in |
|---|---|---|---|---|
| fenced `bash` / `sh` / `console` blocks | 111 | 145 | 39 | #806 |
| fenced `json` blocks | 63 | 74 | 66 | #806 |
| fenced `rust` blocks: macro bodies (`assert!(glob.is_match(…))`) | 59 | 59 | 59 | #818 |
| fenced `yaml` blocks and the YAML front matter | 33 | 38 | 2 | #806 |
| fenced `toml` blocks | 23 | 17 | 3 | #806 |

## Files

| file | what |
|---|---|
| `themes.tsv` | per shipped theme: its Neovim original as `owner/repo@commit`, the colorscheme, the ex commands before it (`set background=light`, `let ayucolor = 'mirage'`), the plugins it needs (bluloco: lush.nvim); `skip` with the reason for a theme with no Neovim original |
| `colors/THEME/LANG.tsv` | LazyVim's foreground of every non-blank character, per file line: `# projects NAME@COMMIT`, `# palette a=c8d3f5 …`, then `PROJECT/PATH<TAB>LINE:RUNS …`, where RUNS is a letter per colour and its count when it repeats |
| `colors/THEME/chrome.tsv` | the highlight groups of the chrome as Neovim draws them: a colour a group leaves unset is `Normal`'s, `reverse` swaps the two |
| `baseline.tsv` | master's characters compared and matching per theme and language |
| `run` | the runner |
| `record.py` | the recording, once |

## The reference

LazyVim as the epic defines it: Neovim with the theme's own colorscheme, nvim-treesitter's
queries (`main`, pinned in `record.py`) and the parsers LazyVim installs by default plus Rust and
Go, built with `cc` at the revisions nvim-treesitter pins, `termguicolors`, `number`,
`cursorline`, `nowrap`, and `vim.treesitter.start` on every buffer; no LSP, so no semantic
tokens. Neovim runs in tmux with every `XDG_*` home and `XDG_CONFIG_DIRS` / `XDG_DATA_DIRS` under the
cache, so neither `~/.config/nvim` nor a system-wide `site` directory reaches it.

The files: four per language from the projects of `tools/d-bench/projects.tsv` at their pinned
commits (Python from paperless-ngx, Rust from ripgrep, TypeScript from outline, Go from caddy,
Markdown from those four), taken in the order of the hash of their path, each with at least 80
lines among its first 200 that are ASCII and at most 140 columns: those are the lines recorded,
so both programs draw all of them and each character has one cell.

## Recording, once

Never in CI. Neovim 0.12, tmux and a C compiler; everything else goes into the cache
(`$THEME_BENCH_CACHE`, default `~/.cache/merl-theme-bench`), the projects into the d-bench's
(`$D_BENCH_CACHE`, default `~/.cache/merl-d-bench`).

```sh
tools/theme-bench/record.py --check                 # every theme of themes.tsv against its shipped tmTheme
tools/theme-bench/record.py --jobs 2                # every theme: 9 min
tools/theme-bench/record.py --theme nord --lang go
```

`--check` loads each theme with its row's commands, as `tools/port-theme.sh` would, without the
parsers, and compares its `Normal` with the
shipped `themes/NAME.tmTheme`'s background and foreground: a difference means the row is not what
the theme was ported from, or its repo changed since. On 2026-10-11 all 92 rows passed.

`tools/port-theme.sh --all` ports every row again, the tmTheme and the groups file
(`themes/NAME.groups.json`, #801), and 91 tmThemes come out byte for byte as shipped. The one that
does not is `tokyonight-moon`, which ships folke's Sublime extra (`extras/sublime`) rather than a
port: re-ported, it matches LazyVim better in Rust, TypeScript and Markdown but worse in Python
and Go, so it waits for the owner (#812). `shokunin-dark` and `shokunin-light` (a VS Code
JSON) and `lucerna`, `seneca` and `seneca-light` (merl's own palettes, #775) have no Neovim
original and are skipped.
