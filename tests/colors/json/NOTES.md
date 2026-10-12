# JSON: characters that differ from LazyVim

Each line below is skipped by `json_and_jsonc_are_coloured_as_lazyvim`, which fails when a listed
character no longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

Not a colour of the code: the recording's cursor sits on the first `{`, and Neovim's matchparen
draws it and its `}` as `MatchParen`.

- `* package.json:1:1` matchparen under the cursor
- `* package.json:19:1` matchparen under the cursor
- `* broken.json:1:1` matchparen under the cursor
- `* broken.json:6:1` matchparen under the cursor
