# PHP: characters that differ from LazyVim

Each line below is skipped by `php_and_the_html_around_it_are_coloured_as_lazyvim`, which fails
when a listed character no longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

Not a colour of the code: the recording's cursor sits on the first `<` of `<?php`, PHP's ftplugin
adds `<:>` to `matchpairs`, and Neovim's matchparen draws it and the `>` it pairs with as
`MatchParen`.

- `* Ledger.php:1:1` matchparen under the cursor
- `* Ledger.php:43:41` matchparen under the cursor
- `* broken.php:1:1` matchparen under the cursor
- `* broken.php:6:16` matchparen under the cursor

The theme's own queries (#819): github-nvim-theme ships `queries/php/highlights.scm`, which Neovim
appends to nvim-treesitter's (`<?php`, `?>` and `<?=` as `@tag`, a variable's `$` as `@operator`);
merl does not load it.

- `github-light broken.php:1` the theme's PHP queries
- `github-light broken.php:5` the theme's PHP queries
- `github-light broken.php:6` the theme's PHP queries
- `github-light Ledger.php:1` the theme's PHP queries
- `github-light Ledger.php:27` the theme's PHP queries
- `github-light Ledger.php:28` the theme's PHP queries
- `github-light Ledger.php:30` the theme's PHP queries
- `github-light Ledger.php:53` the theme's PHP queries
- `github-light Ledger.php:56` the theme's PHP queries
- `github-light Ledger.php:58` the theme's PHP queries
- `github-light Ledger.php:59` the theme's PHP queries
- `github-light Ledger.php:60` the theme's PHP queries
- `github-light Ledger.php:61` the theme's PHP queries
- `github-light Ledger.php:64` the theme's PHP queries
- `github-light Ledger.php:66` the theme's PHP queries
- `github-light Ledger.php:67` the theme's PHP queries
- `github-light Ledger.php:70` the theme's PHP queries
- `github-light Ledger.php:74` the theme's PHP queries
- `github-light Ledger.php:77` the theme's PHP queries
- `github-light Ledger.php:78` the theme's PHP queries
- `github-light Ledger.php:85` the theme's PHP queries
- `github-light Ledger.php:86` the theme's PHP queries
- `github-light Ledger.php:87` the theme's PHP queries
- `github-light Ledger.php:89` the theme's PHP queries
- `github-light Ledger.php:91` the theme's PHP queries
- `github-light Ledger.php:93` the theme's PHP queries
- `github-light Ledger.php:96` the theme's PHP queries
- `github-light Ledger.php:98` the theme's PHP queries
- `github-light Ledger.php:99` the theme's PHP queries
- `github-light Ledger.php:100` the theme's PHP queries
- `github-light Ledger.php:104` the theme's PHP queries
- `github-light Ledger.php:106` the theme's PHP queries
- `github-light Ledger.php:107` the theme's PHP queries
- `github-light Ledger.php:108` the theme's PHP queries
- `github-light Ledger.php:114` the theme's PHP queries
- `github-light Ledger.php:115` the theme's PHP queries
- `github-light Ledger.php:116` the theme's PHP queries
