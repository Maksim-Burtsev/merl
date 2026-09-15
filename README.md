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
twenty-three lessons, each one done when the key actually did what it says, on a copy in a temporary
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
| T | Pick a theme (live preview) |
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
clipboard" is on; Terminal.app cannot). Paste is the terminal's own Cmd+V; outside edit mode it
types into the `/`, `s` and `:` prompts and picker queries, and navigation ignores it. Ctrl+C is
quit again once you press Esc.

In a git repository the gutter shows what differs from the index, as VS Code's does: green for
added lines, blue for changed ones, red under a line where lines were deleted. The marks come
from `git diff` after every save and reload, so they trail an edit by the autosave delay.

Ctrl+Z and Ctrl+Y undo and redo, per file, for as long as it is open; a run of keystrokes on one
line is one step, as in VS Code. There is no save step: edits reach the disk `autosave_delay_ms` after the last keystroke, and
at once when you leave edit mode, switch files or quit. Ctrl+S saves now. A file that changes on
disk under unsaved edits is neither reloaded nor overwritten: the status bar says so, Ctrl+S keeps
your version and Ctrl+R takes the disk's — VS Code's conflict prompt, with keys. Until then, or
while a save keeps failing, merl stays on the file: another one does not open, and `q` has to be
pressed twice to quit without the edits. Tabs, CRLF line endings and the trailing newline come back
out as they went in; binary files, non-UTF-8 files and files with mixed line endings stay
read-only, and so does a line too long to be shown whole.

## How navigation works

There is no language server and no index: every lookup is a regex over the files found at startup,
run through [ripgrep](https://github.com/BurntSushi/ripgrep)'s library crates. `d` knows the
declaration forms below and searches only where such a definition can live; in any other language,
or when the rules find nothing (a field, an enum variant, a parameter), it falls back to a
whole-word search for the identifier. `u` is that whole-word search, always.

| File | What `d` recognises | Searched |
|---|---|---|
| Python | `def`, `class`, module-level assignment (annotated or not) | every `.py` file |
| Go | `func` with or without a receiver, `type`, `var`/`const`, `:=` | every `.go` file |
| TypeScript / JavaScript | `function`, `class`, `interface`, `type`, `enum`, `namespace`, `const`/`let`/`var` (so arrow functions assigned to a name), class and object-literal methods, properties holding a function, behind `export`/`default`/`declare`/`async` and the member modifiers. Plain fields, destructuring and parameters fall back to the whole-word search. | every `.ts`, `.tsx`, `.js`, `.jsx` and friend: they search each other |
| Rust | `fn`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`, `mod`, `macro_rules!`, `let`, behind any `pub(..)`/`async`/`unsafe`/`const`/`extern`/`default` prefix. `impl` blocks count as uses. | every `.rs` file |
| Java | `class`, `interface`, `enum`, `record`, `@interface`; a method or constructor with a body, an abstract or interface method, a field, behind annotations and modifiers. A method's return type has to be a primitive or a name with a capital in it, so a call does not read as a declaration. | every `.java`, `.kt` and `.kts` file: they search each other |
| Kotlin | `fun` (with the receiver of an extension function), `class`, `interface`, `object`, `enum class`, `typealias`, `val`/`var`, behind `private`/`open`/`data`/`sealed`/`suspend`/`override` and the rest | every `.java`, `.kt` and `.kts` file: they search each other |
| Ruby | `def`, `def self.name`, `class`, `module`, an assignment (a constant, an `@ivar`, a local), `attr_accessor`/`attr_reader`/`attr_writer`, `alias`/`alias_method`. A trailing `?` or `!` is not part of the word, so `d` on `empty?` finds `def empty?`. Rails-style DSL (`scope`, `has_many`) falls back to the whole-word search. | every `.rb`, `.rake`, `.gemspec`, `.podspec`, `.rbi`, `.ru` file and `Rakefile`, `Gemfile`, `Vagrantfile` and friends |
| Shell | `name()` and `function name`, an assignment behind `export`/`declare`/`local`/`readonly`/`typeset` (or bare, and `+=`), `alias` | every `.sh`, `.bash`, `.zsh`, `.ksh` and shell dotfile (`.bashrc`, `.zshrc`, `.profile` and friends) |
| SQL | `CREATE` of a table, view, index, function, procedure, trigger, type, schema, sequence, domain, extension, database, role or user, behind `OR REPLACE`, `TEMP`, `UNLOGGED`, `MATERIALIZED`, `UNIQUE` and `IF NOT EXISTS`, schema-qualified or quoted; a `WITH … AS (` common table expression. Keywords ignore case. Columns fall back to the whole-word search. | every `.sql`, `.psql`, `.pgsql`, `.mysql`, `.ddl` and `.dml` file |
| Makefile, `*.mk` | a target, also one of several before the colon; a variable | every Makefile |
| Terraform | the block behind `var.x`, `module.x`, `local.x`, `data.T.N`, `T.N`; a bare name, as in `.tfvars`, is any block with that label | `.tf` files in the same directory |
| Dockerfile | the `FROM … AS name` stage | the same file |
| YAML | the `&name` anchor, a key that opens a block (compose services, CI jobs) | the same file |

In Makefiles, Terraform, Dockerfiles and YAML a `-` is part of the word under the cursor, and `d`
in Terraform reads the whole dotted address, so it works from anywhere in `aws_s3_bucket.logs.id`.

`D` lists every declaration a single regex can recognise (`def class func function fun type fn
struct enum impl trait interface mod module object record typealias union macro_rules! namespace`,
with `export`/`pub`/`async`/`const`/`extern`/`declare` and the Java and Kotlin modifiers
(`public`/`private`/`final`/`open`/`data`/`sealed`/`suspend`/`override` and friends) as prefixes; a
`static` or `const` counts only unindented or exported, since indented they are locals, and only
when the name follows the keyword, as it does in Rust, JavaScript and Kotlin but not in Java, where
a type stands in between), plus shell functions (`name()`; the `function name` form the single regex
already finds), SQL `CREATE`d objects under the name as written (`public.orders`, not CTEs), Java
methods — told from a call by the return type before the name — Makefile targets, Terraform blocks
by address (`aws_s3_bucket.logs`, `data.T.N`, `module.x`, `var.x`, `output.x`), Dockerfile stages
and YAML anchors, each read only from its own kind of file; recomputed on each press. TypeScript's
class methods are not listed: with neither a keyword nor a type in front, the regex cannot tell
`name(` from a call. Neither are the names a Ruby `attr_accessor` line declares, since one line can
declare several. Searches are smart-case — an all-lowercase query ignores case, one uppercase letter
makes it case-sensitive — and `/` and `s` take full regular expressions.

The file list comes from one `.gitignore`-respecting walk at startup and is not refreshed, so
files created while merl is open show up after a restart. Dotfiles are part of it — `.github/`,
`.env`, `.dockerignore` — and only the `.git`, `.hg` and `.svn` stores are skipped. The open file
itself is watched and reloads on every change on disk, keeping the cursor, the scroll position and
the jump history.

## Themes

Themes ship inside the binary as TextMate `.tmTheme` files, and syntect paints the code with them
(syntax definitions come from [bat](https://github.com/sharkdp/bat)'s set, via `two-face`). They
were picked to sit in for hours: palettes designed as a whole, no neon, and light themes that look
like paper rather than an inverted dark one. The default is `tokyonight-moon`;
[docs/themes.md](docs/themes.md) shows every theme with a screenshot and where it was ported from,
and how to port another.

`T` lists them inside merl and repaints everything in the theme under the cursor as it moves:
Enter keeps it and writes it to the config file below, Esc puts the old one back. `merl --theme
NAME` overrides the config for one run; an unknown name exits with code 1 and lists the valid
ones.

The infrastructure half of a repository is highlighted too: Dockerfiles and `Containerfile` (with
`RUN` lines as shell), compose, Kubernetes and CI YAML, Makefiles, Terraform, nginx, `.env`, TOML,
INI and systemd units, `.dockerignore`, `CODEOWNERS`, Sorbet's `.rbi` files and `Dangerfile`. Helm
templates are read as plain YAML, so their `{{ }}` blocks are not highlighted as a template
language.

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

MIT — see [LICENSE](LICENSE). The syntax definitions come from
[bat](https://github.com/sharkdp/bat) via [two-face](https://github.com/CosmicHorrorDev/two-face).
The ported themes keep their authors' licences, shipped next to them in `themes/` and listed in
[docs/themes.md](docs/themes.md#licences).
