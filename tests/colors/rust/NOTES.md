# Rust: characters that differ from LazyVim

Each line below is skipped by `rust_is_coloured_as_lazyvim`, which fails when a listed character
no longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

The theme's own queries (#819): github-nvim-theme ships `queries/rust/highlights.scm`, which
Neovim appends to nvim-treesitter's (attributes as `@keyword.directive`, `crate`/`self`/`super`
as `@keyword`, a lifetime's `'`, capitalised names as `@type`); merl does not load it.

- `github-light items.rs:2` the theme's Rust queries
- `github-light items.rs:4` the theme's Rust queries
- `github-light items.rs:7` the theme's Rust queries
- `github-light items.rs:9` the theme's Rust queries
- `github-light items.rs:12` the theme's Rust queries
- `github-light items.rs:17` the theme's Rust queries
- `github-light items.rs:24` the theme's Rust queries
- `github-light items.rs:26` the theme's Rust queries
- `github-light items.rs:28` the theme's Rust queries
- `github-light items.rs:29` the theme's Rust queries
- `github-light items.rs:30` the theme's Rust queries
- `github-light items.rs:36` the theme's Rust queries
- `github-light items.rs:37` the theme's Rust queries
- `github-light items.rs:47` the theme's Rust queries
- `github-light items.rs:51` the theme's Rust queries
- `github-light items.rs:57` the theme's Rust queries
