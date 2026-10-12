# JavaScript and JSX: characters that differ from LazyVim

Each line below is skipped by `javascript_and_jsx_are_coloured_as_lazyvim`, which fails when a
listed character no longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

The theme's own queries (#819): github-nvim-theme's `queries/jsx/highlights.scm`, which Neovim
appends to nvim-treesitter's, paints a member expression tag's object as `@type` and its property
as `@variable.member`; merl does not load a theme's queries.

- `github-light app.jsx:11:8-17` `List.Group`
