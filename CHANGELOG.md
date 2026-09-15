# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `d` and `D` in SQL. `d` finds the `CREATE` of a table, view, index, function, procedure,
  trigger, type, schema, sequence, domain, extension, database, role or user — behind
  `OR REPLACE`, `TEMP`, `UNLOGGED`, `MATERIALIZED`, `UNIQUE` and `IF NOT EXISTS`, schema-qualified
  or quoted — and `WITH … AS (` common table expressions, across every `.sql`, `.psql`, `.pgsql`,
  `.mysql`, `.ddl` and `.dml` file; keywords ignore case. `D` lists the `CREATE`d objects under
  the name as written. `.psql`, `.pgsql` and `.mysql` now highlight as SQL. (#17)
- Highlighting for infrastructure files bat's set does not recognise by name: `.dockerignore` and
  `CODEOWNERS` (Git Ignore), `Containerfile` and `Dockerfile.*`, `*.jsonc`, systemd units and
  `.npmrc` (INI), `Procfile` and `yarn.lock` (YAML), `WORKSPACE` and `Tiltfile` (Starlark as
  Python). (#16)
- `d` and `D` in Makefiles, Terraform, Dockerfiles and YAML. `d` finds Makefile targets and
  variables across every Makefile, the Terraform block behind `var.x` / `module.x` / `local.x` /
  `data.T.N` / `T.N` in the same directory, and Dockerfile stages, YAML anchors and block keys
  (compose services, CI jobs) in the same file. `D` lists targets, Terraform addresses, stages and
  anchors. A `-` is part of a word in these files. The tutorial gains a lesson on it. (#16)
- Twenty-one more themes, ported from their Neovim originals: Kanagawa Wave / Dragon / Lotus,
  Rosé Pine / Moon / Dawn, Everforest dark / light, Gruvbox Material dark / light, Catppuccin
  Mocha / Latte, Flexoki dark / light, Melange dark / light, Nordfox, Dawnfox, Dayfox, Tokyonight
  Day and Bluloco Light. `tools/port-theme.sh` ports a colorscheme in one command, and a test
  fails on a theme that paints code, YAML, Dockerfiles, Makefiles or TOML in too few colours. (#18)
- `T` picks a theme: the list repaints merl in the theme under the cursor as it moves, Enter keeps
  it and writes `theme` to `~/.config/merl/config.toml` (the rest of the file stays), Esc puts
  the old one back. Two new tutorial lessons teach it. (#18)

### Changed

- The file tree, `o` and project search include dotfiles (`.github/`, `.env`, `.dockerignore`);
  `.gitignore` still applies and `.git` is skipped. (#16)
- Dockerfiles use bat's `Dockerfile (with bash)` grammar: `RUN` lines are highlighted as shell and
  instruction arguments are no longer drawn in the default colour. (#16)
- All three themes colour the names infrastructure grammars emit: TOML keys and tables, INI
  sections, Terraform attributes, `.env` keys, Makefile and nginx variables, Dockerfile stages
  and image tags, YAML anchors and aliases. (#16)

## [0.3.0] - 2026-09-13

### Added

- `{` / `}` jump to the previous / next paragraph (blank line), like vim; in code that is the
  previous / next function without a parser. (#1)
- A welcome screen when merl starts without a file: the logo in the blues of the icon and the
  keys that work before a file is open. It shrinks to the keys alone, then to a one-line hint,
  when the pane is too small. (#9)
- `merl --tutor`: a vimtutor-style tutorial inside merl. Eighteen lessons over a sample Python
  project bundled in the binary, each one advancing when the key did what the lesson asked; the
  unpacked copy lives in a temporary directory and is removed on exit.

### Changed

- Find in file (`/`) matches literal text, as in VS Code: `migrator(` hits `Migrator()` instead
  of failing as an unclosed regex group. Smart case is unchanged; `s>` still takes a regex. (#5)
- A plain Left / Right on a selection collapses it to its start / end without moving further,
  as in VS Code; Up / Down still move from the cursor.
- The jump history now works like VS Code's: the current stop follows the cursor, so `[` goes
  back to where you were, not to where the last jump landed. A plain move farther than ten
  lines, Ctrl+D / Ctrl+U, `n` / `N` and find all add stops of their own; smaller moves update the
  current one. The history keeps the last fifty stops. Fixes #2.
- Enter in the tree on the file that is already open keeps the cursor where it is.

### Fixed

- A reload that shortened the file no longer crashes merl when a position taken before it is
  read back: Backspace to an empty `/` query, an arrow on a selection, `[` to an older stop.
  `[` onto a stop the file no longer reaches lands on the clamped line and keeps the forward
  history; a stop whose file is gone reports it and leaves the history alone.
- A line longer than 20 000 bytes (minified JS, single-line JSON) is wrapped the same way by
  the renderer and by the cursor arithmetic, so End no longer scrolls the line off the screen.
- Alt+letter over a picker, a prompt or the help only closes it; the letter is dropped instead
  of running a normal-mode binding (Alt+q used to quit).
- `u`, `d` and `s` search the open file even when the startup walk skipped it (hidden or
  ignored path opened by name).
- A result list cut at 5 000 hits says `(first 5000)` in the picker title.
- An invalid regex in `s>` is reported as `bad pattern`, not as `no results`.
- `?` scrolls with Up / Down when the terminal is too short for the whole list.
- The status bar says `no auto-reload` when no file watcher could be started, and `file gone`
  when the open file disappears from disk.
- Lines above a selection were painted with the selection background to the right edge.
- The usages, definitions and `s>` search pickers draw each hit with the syntax colours of the
  line it quotes, the same ones the code view shows after jumping there. Only the rows on
  screen are highlighted, so a picker over thousands of hits stays cheap. Fixes #7.
- A jump that went nowhere (`:` with the current line, for instance) erased the forward history.

## [0.2.0] - 2026-09-13

### Added

- Ctrl+D / Ctrl+U move half a screen, cursor and viewport together (vim/less style).
- A VS Code-style selection, painted with the theme's `selection` colour, that runs from where
  the first extending key was pressed to the cursor: Shift+Up / Shift+Down extend it by a line,
  Alt+Shift+Left / Right by a word, Ctrl+Shift+Left / Right to the start / end of the line (what
  Cmd+Shift+Left / Right does in VS Code, since Cmd never reaches a terminal program). Any other
  cursor move, Esc or opening a file clears it.

### Changed

- Shift+Up / Shift+Down no longer jump three lines; Ctrl+D / Ctrl+U cover fast movement.

### Fixed

- Go to definition finds annotated Python assignments (`NAME: Final[int] = ...`).
- Emptying the find query clears the previous highlights and returns to the anchor.

## [0.1.1] - 2026-09-12

### Fixed

- Symbol picker recognises `function` and declarations behind `export`, `pub`, `async` and similar prefixes (TypeScript, JavaScript, Rust `pub fn` were mostly missing).

## [0.1.0] - 2026-09-12

### Added

- Read-only file viewer with a line-number gutter and soft wrap at pane width.
- Syntax highlighting (syntect, bat syntax set) with tokyonight-moon, shokunin-light,
  shokunin-dark themes.
- CLI: `merl`, `merl DIR`, `merl FILE`, `merl FILE:LINE`, `--theme NAME`, `--version`.
- Project root resolution: the given directory, else the file's git toplevel, else its directory.
- Cursor movement: arrows, Shift+Up/Down (3 lines), Shift+Left/Right (word jump), PgUp/PgDn,
  Home/End, Ctrl+Home/Ctrl+End, with a sticky target column.
- Go to line via `:` or Ctrl+G; `q` and Ctrl+C quit, Esc closes the prompt.
- Status bar showing the path relative to the project root, the cursor position, and the focused pane.
- File tree in the left pane: `t` toggles it, Tab switches focus, Enter opens or expands,
  Left/Right collapse and expand; the tree reveals whatever file is opened.
- Fuzzy file picker (nucleo) on `o` / Ctrl+E, with matched characters highlighted.
- Find in file with smart-case regex: `/` or Ctrl+F searches as you type, `n` / `N` step through
  the matches with wraparound, and every match on screen is highlighted.
- Project search, go to definition (Python, Go), symbols and usages via ripgrep's library crates:
  `s`, `d` / F12, `D`, `u` / Shift+F12.
- Jump history: `[` and `]` walk back and forward through the positions a jump left behind.
- Config file `~/.config/merl/config.toml` with a `theme` key.
- Auto-reload: the open file is re-read when it changes on disk, keeping the cursor, the
  scroll position, the jump history and the find pattern.
- Help overlay on `?`, listing every binding; Esc in normal mode clears the find highlights.

[Unreleased]: https://github.com/Maksim-Burtsev/merl/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.3.0
[0.2.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.2.0
[0.1.1]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.1.1
[0.1.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.1.0
