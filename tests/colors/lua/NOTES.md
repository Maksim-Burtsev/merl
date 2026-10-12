# Lua: characters that differ from LazyVim

Each line below is skipped by `lua_is_coloured_as_lazyvim`, which fails when a listed character no
longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

The pattern of `find`, `match`, `gmatch` and `gsub` is parsed as `luap` (Lua patterns) through an
injection in Neovim; merl colours it as one `@string.regexp`. tree-sitter-luap's only crates.io
release and the commit nvim-treesitter pins both depend on tree-sitter 0.20: its `Language` is not
0.27's, and the crate would compile a second tree-sitter runtime into merl.

- `* flow.lua:29:31-33` `%a+`
- `* flow.lua:32:39-50` `^%s*(.-)%s*$`
