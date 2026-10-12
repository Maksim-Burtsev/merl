# TypeScript: characters that differ from LazyVim

Each line below is skipped by `typescript_is_coloured_as_lazyvim`, which fails when a listed
character no longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

Injections (#818): LazyVim parses these ranges with another grammar, merl draws them with
TypeScript's. The `jsdoc` comment on `classes.ts:83` draws the same either way in these themes.

- `* classes.ts:84:20-23` the regex literal's pattern, `regex`
