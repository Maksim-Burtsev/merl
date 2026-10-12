# Bash: characters that differ from LazyVim

Each line below is skipped by `bash_is_coloured_as_lazyvim`, which fails when a listed character
no longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

The theme's own queries (#819): github-nvim-theme ships `queries/bash/highlights.scm`, which
Neovim appends to nvim-treesitter's (`$(`, `<(` and their `)` as `@string`, `#` of an expansion as
`@operator`); merl does not load it.

- `github-light script.sh:18:7-8` `$(` the theme's Bash queries
- `github-light script.sh:18:54` `)` the theme's Bash queries
- `github-light script.sh:21:22` `#` in `${#targets[@]}`, the theme's Bash queries
- `github-light script.sh:42:8-9` `<(` the theme's Bash queries
- `github-light script.sh:42:22` `)` the theme's Bash queries
- `github-light script.sh:47:25-26` `$(` the theme's Bash queries
- `github-light script.sh:47:35` `)` the theme's Bash queries
