# TypeScript: characters that differ from LazyVim

Each line below is skipped by `typescript_is_coloured_as_lazyvim`, which fails when a listed
character no longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

None: the regex literal (`classes.ts:84`) is parsed as `regex` through injections (#803), and
the `jsdoc` comment on `classes.ts:83` draws the same as with TypeScript's captures in these themes.
