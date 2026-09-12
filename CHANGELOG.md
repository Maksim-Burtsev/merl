# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- Jump history: `[` and `]` walk back and forward through the positions a jump left behind.
- Config file `~/.config/merl/config.toml` with a `theme` key.

[Unreleased]: https://github.com/Maksim-Burtsev/merl/compare/HEAD...HEAD
