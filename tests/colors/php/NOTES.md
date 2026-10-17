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
