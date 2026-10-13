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

Lua (#806), with its injections (`---@` comments parsed as `luadoc`, `string.format` strings as
`printf`); the bench's four files are redis's `deps/lua`, the only Lua of the d-bench projects,
so the language takes files of 25 recordable lines rather than 80:

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Lua | 4,590 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

- The pattern of `find`, `match`, `gmatch` and `gsub`: Neovim parses it as `luap`, whose crates
  build against tree-sitter 0.20; merl colours it as one `@string.regexp`
  (`tests/colors/lua/NOTES.md`). None of the bench's files has one.

C (#806), `.c` files only (Neovim opens a `.h` as C++, as merl does), with its injections
(`printf` in the format strings of the `printf` family; `comment`, `doxygen` and `re2c` comments
stay plain, as LazyVim installs no parser for them):

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| C | 17,618 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

C++ (#807), `.cc`, `.cpp` and the other extensions Neovim opens as `cpp`, and `.h` unless Neovim's
`detect.header` makes it Objective-C++ from its first 200 lines, with C's queries and injections
inherited (`; inherits: c`); `doxygen` comments stay plain, as LazyVim installs no parser for them:

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| C++ | 19,079 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

Java (#807), `.java` (`.bsh`, which syntect calls Java and Neovim does not, stays on syntect), with
its injections (`printf` in the first string of a `format` or `printf` call and in the receiver of
`formatted`; `comment` and `javadoc` comments stay plain, as LazyVim installs no parser for them):

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Java | 15,213 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

C# (#807), `.cs` and `.csx`, and a Markdown block marked `cs` or `csharp`; `comment` comments stay
plain, as LazyVim installs no parser for them:

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| C# | 16,671 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

Kotlin (#807), `.kt` and `.kts`, with its injections (`regex` in the string of `toRegex()`,
`Regex(…)` and `Regex.fromLiteral(…)`, `printf` in the receiver of `format`; `comment` comments stay
plain, as LazyVim installs no parser for them):

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Kotlin | 18,781 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

Swift (#807), `.swift`, with its injections (`regex` in a regex literal; `comment` comments stay
plain, as LazyVim installs no parser for them). nvim-treesitter generates its `parser.c` from the
pinned `grammar.json` (`generate = true`), so `record.py` runs `tree-sitter generate` for it; merl
takes tree-sitter-swift 0.7.4 from crates.io, published from that same commit with the file
generated:

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Swift | 22,782 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

Ruby (#808), the files Neovim opens as `ruby`: `.rb`, `.rbi`, `.rbw`, `.rake`, `.gemspec`, `.ru`,
`.builder`, `.rjs`, `.rxml`, `.rant`, `Gemfile`, `Rakefile`, `Brewfile`, `Vagrantfile` and the like
(`Dangerfile`, `Podfile`, `.thor` and the rest syntect calls Ruby and Neovim does not stay on
syntect), with its injections (`regex` in a regex literal, a heredoc's body by its delimiter;
`comment` and `rbs` comments stay plain, as LazyVim installs no parser for them):

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Ruby | 13,458 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

HTML and CSS (#806). An HTML file is parsed as `html` unless Neovim's `detect.html` makes it
`htmlangular`, `xhtml`, `htmldjango` or `superhtml` from its first 40 lines (merl leaves those to
syntect, the bench skips them); `<style>` and `style="…"` are CSS, `<script>` and `on…="…"` are
JavaScript, `<script type="text/x">` is `x` through `#gsub!`, an importmap is JSON:

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| HTML | 22,445 | 100.0 alabaster-light | 100.0 | 100.0 vscode-light |
| CSS | 16,766 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

XML (#806), `.xml`, `.svg`, `.xsd` and `.xslt` as nvim-treesitter registers them, unless Neovim's
`detect.xml` makes the file DocBook or XBL from its first 100 lines; the bench's four files are
the only ones of 80 recordable lines in the d-bench projects:

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| XML | 20,823 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

Diff (#806), `.diff` and `.patch`, unless Neovim's `detect.patch` makes a `.patch` whose first line
is a `git format-patch` header `gitsendemail`, or its `detect.dep3patch` makes a file under
`/debian/patches/` with a DEP3 header `dep3patch` (merl leaves those to syntect, the bench skips the
first; the d-bench projects hold none of the second); `.rej`, `diff` in Neovim, is plain text in
syntect and stays so. A capture's background is dropped as for every language, unless the group is
`reverse`, where the background is the text's colour (nord's `@diff.plus`, `@diff.minus`,
`@diff.delta`); the themes that give `@diff.*` a background tint (tokyonight, github, gruvbox and
others) draw it in LazyVim and not in merl (#845), which the bench, comparing foregrounds, does not count:

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Diff | 18,764 | 100.0 adwaita | 100.0 | 100.0 vscode-light |

Query (#806), a `.scm` file under a `queries` directory as Neovim's `/queries/.*%.scm$` makes it
`query` (any other `.scm` stays Scheme on syntect), and a Markdown block marked `query`; `regex` is
injected into `#match?` and its kin, `#lua-match?` and `#gsub!` patterns are `luap` in Neovim and
stay one string in merl (`tests/colors/query/NOTES.md`; none in the bench's files), `comment`
stays plain. No d-bench project has query files, so the bench takes them from nvim-treesitter at
`NVIM_TS` (`THEME_ONLY` in `record.py`):

| language | chars | lowest | median | highest |
|---|---|---|---|---|
| Query | 7,948 | 100.0 alabaster-light | 100.0 | 100.0 minischeme-light |

Vim (#806) stays on syntect: the commit nvim-treesitter pins (039c8d0) has no `Cargo.toml`, so
it cannot be a git dependency, and crates.io's only release, 0.4.0, has another `parser.c` and
binds `tree_sitter::Language` through the pre-0.24 `extern "C"` API instead of `tree-sitter-language`. No d-bench project has a `.vim` file either.

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
JavaScript from eslint, JSX from mastodon, Go from caddy, Lua and C from redis, C++ from leveldb, Java from halo, C# from eShop, Kotlin from nowinandroid, Swift from Alamofire, Ruby from mastodon, HTML from bootstrap, caddy, eslint, outline and redis, CSS from gitea,
paperless-ngx, caddy and redis, XML from koel, paperless-ngx and nowinandroid, Diff from rules_go, openzeppelin-contracts, outline and halo, Query from nvim-treesitter, Markdown from ripgrep, caddy,
outline and paperless-ngx, Bash, JSON, YAML and TOML from redis, paperless-ngx, ripgrep, caddy and eslint
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
