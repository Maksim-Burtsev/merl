# Query: characters that differ from LazyVim

Each line below is skipped by `query_is_coloured_as_lazyvim`, which fails when a listed character no
longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

The pattern of `#lua-match?` (and of `#gsub!`) is parsed as `luap` (Lua patterns) through an
injection in Neovim; merl colours it as the string it is in. tree-sitter-luap's only crates.io
release and the commit nvim-treesitter pins both depend on tree-sitter 0.20, as in
`tests/colors/lua/NOTES.md`.

- `* queries/highlights.scm:13:40-58` `^/[*][*][^*].*[*]/$`
