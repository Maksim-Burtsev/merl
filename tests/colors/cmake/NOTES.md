# CMake: characters that differ from LazyVim

Each line below is skipped by `cmake_is_coloured_as_lazyvim`, which fails when a listed character
no longer differs. `THEME FILE:LINE[:COLS]`, `*` for every theme.

None.

`broken.cmake` opens with a bracket comment on purpose (#876). tree-sitter-cmake 0.7.5's
`external_scanner_create` takes its state with `malloc` and never sets `token`, and deserializing
an empty state resets only `level`. Until the first bracket token sets `token`, error recovery
reads whatever the heap held: zero on macOS, which frees memory zeroed, so `[[` content is
expected and `set` and `endif` turn into other nodes; leftovers on glibc. Neovim draws the same
as merl on the same heap: `MallocScribble=1` (freed memory filled with 0x55) makes both draw the
x86_64 Linux colours on macOS. The bracket comment sets `token` before the first error, so the
file draws the same on every heap. Without it and with the macOS recording (commit 37b29406),
`MallocScribble=1 cargo test cmake_is_coloured_as_lazyvim` shows the 78 characters x86_64 Linux
got.
