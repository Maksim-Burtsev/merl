# Rust: characters that differ from LazyVim

Each line below is skipped by `rust_is_coloured_as_lazyvim`, which fails when a listed character
no longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

Injections (#818): LazyVim parses these ranges with another grammar, merl draws them with Rust's.

- `* exprs.rs:14:27-40` the `Regex::new` pattern, `regex`
- `* exprs.rs:36:49-51` `add(1, 2)` inside `println!`, the macro body parsed as Rust
- `* exprs.rs:38:16-20` `moved()` inside `assert_eq!`, the macro body parsed as Rust
- `* exprs.rs:39:16-23` `re.is_match(..)` inside `assert!`, the macro body parsed as Rust

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
