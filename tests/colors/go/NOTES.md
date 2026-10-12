# Go: characters that differ from LazyVim

Each line below is skipped by `go_is_coloured_as_lazyvim`, which fails when a listed character no
longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

Injections (#818): LazyVim parses these ranges with another grammar, merl draws them with Go's.

- `* flow.go:10:32-42` the `regexp.MustCompile` pattern, `regex`
- `* flow.go:21:15-16` `%d` in `fmt.Printf`, `printf`
- `* flow.go:21:25-26` `%s` in `fmt.Printf`, `printf`
- `* flow.go:21:29-30` `%v` in `fmt.Printf`, `printf`
- `* flow.go:23:30-31` `%q` in `fmt.Errorf`, `printf`
- `* flow.go:53:32-33` `%T` in `fmt.Fprintf`, `printf`
