# merl

Read-only code navigator for the terminal — VS Code's reading half, without the window.

[![CI](https://img.shields.io/github/actions/workflow/status/Maksim-Burtsev/merl/ci.yml?branch=master&label=ci)](https://github.com/Maksim-Burtsev/merl/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Maksim-Burtsev/merl)](https://github.com/Maksim-Burtsev/merl/releases)
[![License](https://img.shields.io/github/license/Maksim-Burtsev/merl)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)

## What it is

merl opens a codebase read-only: there is no text input, no undo, no clipboard, and nothing it can
save over. It is keyboard-only and terminal-agnostic — no mouse, no Cmd, no Alt, so the same
bindings work over SSH, in tmux, and in whatever terminal you happen to have. It exists for
reviewing code and walking a codebase next to a coding agent, where you read far more than you type.

## Install

- Homebrew: TODO (`brew install Maksim-Burtsev/tap/merl`)
- Prebuilt binaries: TODO (GitHub Releases)
- From source: TODO (`cargo install --git https://github.com/Maksim-Burtsev/merl`)

## Usage

```
merl                 # the current directory
merl DIR             # a project directory
merl FILE            # a single file
merl FILE:LINE       # a file, positioned at a line
merl --theme NAME    # override the configured theme
merl --version
```

## Keys

| Action | Key | VS Code alias |
|---|---|---|
| Open file (fuzzy) | `o` | Ctrl+E |
| Find in file / next / prev | `/`, `n`, `N` | Ctrl+F |
| Search project | `s` | — |
| Go to definition (word under cursor) | `d` | F12 |
| Project symbols (fuzzy) | `D` | — |
| Usages (word under cursor) | `u` | Shift+F12 |
| Back / forward in jump history | `[` / `]` | — |
| Toggle tree / switch focus | `t` / Tab | — |
| Move cursor | arrows | |
| 3 lines / word jump | Shift+Up/Down / Shift+Left/Right | |
| Page / line start-end / file start-end | PgUp PgDn / Home End / Ctrl+Home Ctrl+End | |
| Go to line | `:` | Ctrl+G |
| Help / quit / close overlay | `?` / `q` (and Ctrl+C) / Esc | |
| Tree: move / open / collapse-expand | Up/Down / Enter / Left/Right | |
| Picker: move / accept / cancel | Up/Down (Ctrl+N/P) / Enter / Esc | |

## Themes

`tokyonight-moon` (default), `shokunin-light`, `shokunin-dark`. TODO: the themes are still
placeholders; embedded `.tmTheme` files and syntax highlighting land next.

## Config

`~/.config/merl/config.toml`, one key:

```toml
theme = "tokyonight-moon"
```

An unknown theme name exits with code 1 and lists the valid ones.

## Why not vim/helix/micro

Those are editors: their reading features sit behind a modal editing model you have to know first.
merl drops editing entirely, so the whole keymap is the navigation keymap.

## Status

v0.1, in progress. Not yet released.

## License

MIT — see [LICENSE](LICENSE).
