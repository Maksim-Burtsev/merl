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

Go and Rust (#805), with their injections (#818: `printf` and `regex` in Go, macro bodies parsed
again as Rust and `regex` in Rust):

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Go | 14,810 | 100.0 adwaita | 100.0 | 100.0 vscode-light |
| Rust | 17,576 | 96.3 github-dark | 100.0 | 100.0 vscode-light |

- The theme's own queries (#819): github-dark and github-light append github-nvim-theme's
  `queries/rust/highlights.scm` (attributes, `crate`/`self`/`super`, lifetimes, capitalised
  names), which merl does not load.

Markdown (#803), 18,398 characters per theme: 100.0 % median (100.0 adwaita, 100.0
vscode-light), against 71.3 % with the setting off. Every character of Markdown's own grammars,
`markdown` and `markdown_inline`, and of the Python, Go, Rust, Bash, JSON, YAML and TOML its
fences hold, matches in the themes read by cause (tokyonight-moon, dayfox, github-light,
alabaster-light): no miss is left to list.

Bash, JSON, YAML and TOML (#806), with Bash injected into YAML's `run:` blocks:

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Bash | 12,491 | 99.4 github-dark | 100.0 | 100.0 vscode-light |
| JSON | 7,070 | 99.9 alabaster-light | 99.9 | 100.0 vscode-light |
| YAML | 16,564 | 99.8 github-dark | 100.0 | 100.0 vscode-light |
| TOML | 10,487 | 99.9 alabaster-light | 99.9 | 100.0 vscode-light |

- The theme's own queries (#819): github-dark and github-light append github-nvim-theme's
  `queries/bash/highlights.scm` (`$(`, `<(` and their `)` as strings, the `#` of `${#a}` as an
  operator), in Bash files and in YAML's `run:` blocks, which merl does not load.
- The recording's cursor: a file starting with a bracket (every JSON file, `[tool]` in TOML) has
  its first `{` or `[` and the matching one drawn as `MatchParen` by Neovim, not a colour of the
  code: two characters per file.

TypeScript, TSX, JavaScript and JSX (#804), with #803's injections (regex literals parsed as
`regex`, `/** */` comments as `jsdoc` since #818; an injected language merl has no grammar for,
such as `t` in ``t(`…`)``, is left plain as in Neovim, the syntect fallback being only for
Markdown's code blocks):

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| TypeScript | 13,701 | 100.0 adwaita | 100.0 | 100.0 vscode-light |
| TSX | 9,700 | 99.7 github-dark | 100.0 | 100.0 vscode-light |
| JavaScript | 13,199 | 100.0 adwaita | 100.0 | 100.0 vscode-light |
| JSX | 14,130 | 100.0 | 100.0 | 100.0 |

- The theme's own queries (#819): github-dark and github-light append github-nvim-theme's
  `queries/jsx/highlights.scm` (a member expression tag such as `<Icons.Close>`), which merl does
  not load.

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
queries and plugin (`main`, pinned in `record.py`; the plugin maps `typescriptreact` to `tsx`,
`javascriptreact` to `javascript`, `sh` to `bash` and `jsonc` to `json`, #804, #806) and the parsers LazyVim installs by default plus Rust and
Go, built with `cc` at the revisions nvim-treesitter pins, `termguicolors`, `number`,
`cursorline`, `nowrap`, and `vim.treesitter.start` on every buffer; no LSP, so no semantic
tokens. Neovim runs in tmux with every `XDG_*` home and `XDG_CONFIG_DIRS` / `XDG_DATA_DIRS` under the
cache, so neither `~/.config/nvim` nor a system-wide `site` directory reaches it.

The files: four per language from the projects of `tools/d-bench/projects.tsv` at their pinned
commits (Python from paperless-ngx, Rust from ripgrep, TypeScript and TSX from outline,
JavaScript from eslint, JSX from mastodon, Go from caddy, Markdown from ripgrep, caddy, outline
and paperless-ngx, Bash, JSON, YAML and TOML from redis, paperless-ngx, ripgrep, caddy and eslint
as `LANGS` in `record.py` lists them), taken in the order of the hash of their path, each with at least 80
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
