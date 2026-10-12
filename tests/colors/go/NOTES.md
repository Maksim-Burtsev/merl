# Go: characters that differ from LazyVim

Each line below is skipped by `go_is_coloured_as_lazyvim`, which fails when a listed character no
longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

None: the `regexp.MustCompile` pattern and the `%` verbs of `Printf`-style strings are parsed as
`regex` and `printf` through injections (#818).
