# merl

Read-only code navigator for the terminal — VS Code's reading half, without the window.

[![CI](https://img.shields.io/github/actions/workflow/status/Maksim-Burtsev/merl/ci.yml?branch=master&label=ci)](https://github.com/Maksim-Burtsev/merl/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Maksim-Burtsev/merl)](https://github.com/Maksim-Burtsev/merl/releases)
[![License](https://img.shields.io/github/license/Maksim-Burtsev/merl)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)

merl opens a codebase read-only: no text input, no undo, no clipboard, nothing it can save over.
It is keyboard-only and terminal-agnostic — no mouse, no Cmd, no Alt — so the same bindings work
over SSH, in tmux, and in whatever terminal you happen to have. It is for reviewing code and
walking a codebase in a terminal split next to a coding agent, where you read far more than you
type and the file under you keeps changing.

## Demo

<!-- TODO: screenshot -->

## Install

Homebrew:

```sh
brew install maksim-burtsev/tap/merl
```

Prebuilt binaries for macOS (arm64) and Linux (x86_64, arm64) are attached to every
[release](https://github.com/Maksim-Burtsev/merl/releases):

```sh
curl -fsSL https://github.com/Maksim-Burtsev/merl/releases/latest/download/merl-aarch64-apple-darwin.tar.gz | tar xz
sudo mv merl /usr/local/bin/
```

From source (Rust 1.85 or newer, for the 2024 edition):

```sh
cargo install --git https://github.com/Maksim-Burtsev/merl
```

## Usage

```
merl                 # the current directory
merl DIR             # a project directory
merl FILE            # a single file
merl FILE:LINE       # a file, positioned at a line
merl --theme NAME    # override the configured theme
merl --version
```

The `FILE:LINE` form is what compilers, linters and grep already print, so a result can be pasted
straight in:

```sh
merl path/to/file.py:120
```

## Keys

| Key | Action |
|---|---|
| o / Ctrl+E | Open a file (fuzzy) |
| / / Ctrl+F | Find in the open file |
| n / N | Next / previous match |
| s | Search the project |
| d / F12 | Go to definition of the word under the cursor |
| D | Project symbols (fuzzy) |
| u / Shift+F12 | Usages of the word under the cursor |
| [ / ] | Back / forward in the jump history |
| : / Ctrl+G | Go to line |
| t | Show or hide the file tree |
| Tab | Switch focus between tree and code |
| Arrows | Move the cursor |
| Shift+Up / Shift+Down | Move three lines |
| Shift+Left / Shift+Right | Move one word |
| Ctrl+D / Ctrl+U | Move half a screen down / up |
| PgUp / PgDn | Move one screen |
| Home / End | Start / end of the line |
| Ctrl+Home / Ctrl+End | Start / end of the file |
| Esc | Close an overlay, or clear the find highlights |
| ? | This help |
| q / Ctrl+C | Quit |
| Tree: Up / Down | Move |
| Tree: Enter | Open the file, or expand the directory |
| Tree: Left / Right | Collapse / expand |
| Picker: Up / Down, Ctrl+P / Ctrl+N | Move |
| Picker: Enter | Accept |
| Picker: Esc | Cancel |

`?` shows the same table inside merl.

## How navigation works

There is no language server and no index: every lookup is a regex over the files found at startup,
run through [ripgrep](https://github.com/BurntSushi/ripgrep)'s library crates. `d` knows Python
(`def`/`class`, module-level assignment) and Go (`func` with or without a receiver, `type`,
`var`/`const`, `:=`) and searches only files with the same extension; in any other language it
falls back to a whole-word search for the identifier. `u` is that whole-word search, always. `D`
lists every declaration a single regex can recognise (`def class func function type fn struct
enum impl trait interface`, with `export`/`pub`/`async` prefixes), recomputed on each press. Searches are smart-case — an all-lowercase query
ignores case, one uppercase letter makes it case-sensitive — and `/` and `s` take full regular
expressions.

The file list comes from one `.gitignore`-respecting walk at startup and is not refreshed, so
files created while merl is open show up after a restart. The open file itself is watched and
reloads on every change on disk, keeping the cursor, the scroll position and the jump history.

## Themes

Three themes ship inside the binary as TextMate `.tmTheme` files, and syntect paints the code
with them (syntax definitions come from [bat](https://github.com/sharkdp/bat)'s set, via
`two-face`):

| Name | |
|---|---|
| `tokyonight-moon` | default, dark |
| `shokunin-light` | light |
| `shokunin-dark` | dark |

Pick one with `merl --theme NAME`, or set `theme` in the config file below. An unknown name exits
with code 1 and lists the valid ones.

`tokyonight-moon` is converted from [folke/tokyonight.nvim](https://github.com/folke/tokyonight.nvim)
(Apache-2.0, see [themes/LICENSE-tokyonight](themes/LICENSE-tokyonight)). The two Shokunin themes
are generated from the VS Code JSON sources in `tools/themes-src/` with
`python3 tools/vscode2tmtheme.py in.json out.tmTheme "Name"`.

## Config

`~/.config/merl/config.toml`, one key:

```toml
theme = "tokyonight-moon"
```

## Terminals

merl runs in any terminal. Where the kitty keyboard protocol is offered (Ghostty, kitty, WezTerm,
iTerm2 3.5+, foot, agterm) it is used, which makes every modified key unambiguous; everywhere else
merl falls back to the legacy escape sequences. Known limits: Terminal.app on macOS sends neither
Shift+arrows nor Ctrl+Home, and F12 on Mac keyboards needs Fn — which is why `d` and `u` are the
primary keys and the function keys only aliases.

## Why not vim / helix / micro

Those are editors: their reading features sit behind a modal editing model or a keymap of their
own that you have to learn first. merl is a reader with VS Code-shaped habits — arrows, Home/End,
Ctrl+E, F12 — and nothing new to memorise. If you already live in vim, you do not need this.

## Status

v0.1, a personal tool made public. Issues are welcome; pull requests may wait.

## License

MIT — see [LICENSE](LICENSE). The `tokyonight-moon` theme is converted from
[folke/tokyonight.nvim](https://github.com/folke/tokyonight.nvim) (Apache-2.0); the syntax
definitions come from [bat](https://github.com/sharkdp/bat) via
[two-face](https://github.com/CosmicHorrorDev/two-face).
