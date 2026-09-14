<p align="center">
  <img src="assets/icon.png" width="128" alt="merl">
</p>

<h1 align="center">merl</h1>

<p align="center">Code navigator for the terminal — VS Code's reading half, without the window.</p>

<p align="center">
  <a href="https://github.com/Maksim-Burtsev/merl/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Maksim-Burtsev/merl/ci.yml?branch=master&label=ci" alt="CI"></a>
  <a href="https://github.com/Maksim-Burtsev/merl/releases"><img src="https://img.shields.io/github/v/release/Maksim-Burtsev/merl" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Maksim-Burtsev/merl" alt="License"></a>
  <a href="https://doc.rust-lang.org/edition-guide/rust-2024/index.html"><img src="https://img.shields.io/badge/rust-2024%20edition-orange" alt="Rust 2024 edition"></a>
</p>

merl opens a codebase and gets you to the right line fast: fuzzy file open, project search,
go to definition and usages, all on VS Code-shaped keys. It is keyboard-only and
terminal-agnostic — no mouse, no Cmd — so the same bindings work over SSH, in tmux, and in
whatever terminal you happen to have. It is for reviewing code and walking a codebase in a
terminal split next to a coding agent, where you read far more than you type and the file under
you keeps changing. Today it only reads; editing is
[planned](https://github.com/Maksim-Burtsev/merl/issues/11).

## Demo

<p align="center">
  <img src="assets/demo.gif" alt="merl on a small Python project: fuzzy-open a file, find, go to definition, back, usages, project symbols, and the key help" width="900">
</p>

The project is the one `merl --tutor` uses. The recording is scripted in [`assets/demo.tape`](assets/demo.tape)
for [vhs](https://github.com/charmbracelet/vhs), so it can be regenerated after a UI change.

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
merl --tutor         # interactive tutorial, ~10 minutes
merl --version
```

`merl --tutor` walks through every navigation key on a small Python project bundled in the binary:
eighteen lessons, each one done when the key actually did what it says, on a copy in a temporary
directory that is removed when you quit.

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
| Enter | Edit at the cursor (Esc returns to navigation) |
| Ctrl+S | Save now (edits are saved on their own after a pause) |
| Ctrl+R | Reload from disk, dropping unsaved edits |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Edit: Ctrl+C / Ctrl+X | Copy / cut the selection, or the line, to the clipboard |
| Arrows | Move the cursor |
| Shift+Up / Shift+Down | Extend the selection by a line |
| Shift+Left / Shift+Right | Move one word |
| Alt+Shift+Left / Right | Extend the selection by a word |
| Ctrl+Shift+Left / Right | Extend the selection to the start / end of the line |
| Ctrl+D / Ctrl+U | Move half a screen down / up |
| { / } | Previous / next paragraph (blank line) |
| PgUp / PgDn | Move one screen |
| Home / End | Start / end of the line |
| Ctrl+Home / Ctrl+End | Start / end of the file |
| Esc | Close an overlay, leave edit mode, or clear selection and find |
| ? | This help |
| q / Ctrl+C | Quit |
| Tree: Up / Down | Move |
| Tree: Enter | Open the file, or expand the directory |
| Tree: Left / Right | Collapse / expand |
| Picker: Up / Down, Ctrl+P / Ctrl+N | Move |
| Picker: Enter | Accept |
| Picker: Esc | Cancel |
| Picker: PgUp / PgDn | Move one page |
| Help: Up / Down | Scroll |

`?` shows the same table inside merl.

## Editing

Enter turns the cursor into a text cursor, Esc turns it back. In between, merl is a plain
editor with VS Code habits: letters insert, Enter splits the line and keeps its indentation, Tab
indents the way the file already does (tabs or four spaces, shown in the status bar), arrows and
Home / End move, Shift+arrows select. The letter commands are letters again once you press Esc;
the chord aliases (Ctrl+E, Ctrl+F, Ctrl+G, F12) work while editing.

Typing over a selection replaces it, Backspace and Delete remove it. Ctrl+C and Ctrl+X copy and
cut the selection (or the whole line without one) to the system clipboard through the terminal
(OSC 52: Ghostty, kitty, WezTerm, agterm, and iTerm2 once "Applications in terminal may access
clipboard" is on; Terminal.app cannot). Paste is the terminal's own Cmd+V. Ctrl+C is quit again
once you press Esc.

Ctrl+Z and Ctrl+Y undo and redo, per file, for as long as it is open; a run of keystrokes on one
line is one step, as in VS Code. There is no save step: edits reach the disk `autosave_delay_ms` after the last keystroke, and
at once when you leave edit mode, switch files or quit. Ctrl+S saves now. A file that changes on
disk under unsaved edits is not reloaded: the status bar says so, Ctrl+S keeps your version and
Ctrl+R takes the disk's — VS Code's conflict prompt, with keys. Tabs, CRLF line endings and the
trailing newline come back out as they went in; binary and non-UTF-8 files stay read-only.

## How navigation works

There is no language server and no index: every lookup is a regex over the files found at startup,
run through [ripgrep](https://github.com/BurntSushi/ripgrep)'s library crates. `d` knows the
declaration forms of a few languages and searches only files with the same extension (or the
same family: `.ts`, `.tsx`, `.js`, `.jsx` and friends search each other); in any other language,
or when the rules find nothing (a field, an enum variant, a parameter), it falls back to a whole-word search for the identifier. `u` is that whole-word search, always.

| Language | What `d` recognises |
|---|---|
| Python | `def`, `class`, module-level assignment (annotated or not) |
| Go | `func` with or without a receiver, `type`, `var`/`const`, `:=` |
| TypeScript / JavaScript | `function`, `class`, `interface`, `type`, `enum`, `namespace`, `const`/`let`/`var` (so arrow functions assigned to a name), class and object-literal methods, properties holding a function, behind `export`/`default`/`declare`/`async` and the member modifiers. Plain fields, destructuring and parameters fall back to the whole-word search. |
| Rust | `fn`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`, `mod`, `macro_rules!`, `let`, behind any `pub(..)`/`async`/`unsafe`/`const`/`extern`/`default` prefix. `impl` blocks count as uses. |

`D` lists every declaration a single regex can recognise (`def class func function type fn struct
enum impl trait interface mod const static union macro_rules! namespace`, with `export`/`pub`/`async`/`const`/`extern`/`declare`
prefixes), recomputed on each press. Class methods without a keyword in front are not listed:
the regex cannot tell `name(` from a call. Searches are smart-case — an all-lowercase query
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

`~/.config/merl/config.toml`:

```toml
theme = "tokyonight-moon"
autosave_delay_ms = 1000
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

Early, a personal tool made public. Issues are welcome; pull requests may wait.

## License

MIT — see [LICENSE](LICENSE). The `tokyonight-moon` theme is converted from
[folke/tokyonight.nvim](https://github.com/folke/tokyonight.nvim) (Apache-2.0); the syntax
definitions come from [bat](https://github.com/sharkdp/bat) via
[two-face](https://github.com/CosmicHorrorDev/two-face).
