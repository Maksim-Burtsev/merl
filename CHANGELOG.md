# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Shift+Left / Right extend the selection by a char, the step that was missing between a word
  and a line.
- `v` selects the word under the cursor, pressed again the line, then the paragraph between blank
  lines; a selection made by hand grows the same way. The tutorial has a lesson for it.
- Ctrl+C copies the selection in navigation too, with no trip through edit mode. Without a
  selection it still quits. Cmd+C / Cmd+X do the same from a terminal set up to pass them on
  (one Ghostty line, in the README), and Cmd+C never quits.

### Changed

- The word jump moved from Shift+Left / Right to Alt+Left / Right (Option on a Mac), where every
  other editor has it. Esc b / Esc f, which Ghostty, iTerm and Terminal.app send for Option+arrow,
  are the same jump and no longer leave edit mode.

### Fixed

- `d` finds a Python `async def` and a TypeScript method signature with no body
  (`find(id: string): User;` in an interface, an abstract class, an overload or a `.d.ts`). (#69)
- Go lookups outside the project skip `_test.go` files, `testdata` and nested modules such as
  GOROOT's `cmd`, which no import reaches. A relative import (`from . import views`, `./utils`)
  and a Go package whose name its path decorates (`gopkg.in/yaml.v3`, `go-sqlite3`) now bind the
  name they bring in. (#69)
- A TypeScript `import { type Foo } from 'lib'` binds `Foo`, not `type`, so `d` on `Foo` looks in
  `lib`. (#73)
- `d` finds a TypeScript method whose empty body sits on its own line, `close(): void {}`. (#83)
- `d` on a member of a chain that hangs off a call, an index or `?.`, such as
  `make_uow().users.delete_user`, no longer reads a local `users` as the receiver and jumps into
  that variable's type: the word is a member of a value whose type is not known. (#85)
- A Python import no longer counts as a declaration of the modules on its path: with
  `from .guild import Guild` in the file, a parameter `guild` handed on to `self.guild` keeps its
  type, and `d` on `self.guild.x` reads it instead of falling back to the search by name. (#85)
- Eight themes had the selection colour within a few points of the cursor line highlight, so a
  selection inside the cursor line was nearly invisible: rose-pine-moon, rose-pine-dawn, nordfox,
  nightfox, gruvbox-material-light and material-light move the selection a step along their own
  palette, solarized-light darkens it, dracula lightens the line highlight instead. The theme test
  now requires the two colours to be apart.

### Added

- `d` says how it found the target. After a jump the status line reads
  `delete_user → UserRepository.delete_user (by name, 1 match)` or `load: via import json`. Over
  a picker the status line and the title read `delete_user: by name, 2 declarations`, and each row
  starts with the class, interface or receiver the declaration sits in and why it is listed. A
  module lookup that falls back past the module an import names, or a value whose name only
  matches a module, says `by name`. (#69)
- `d` on `x.word` where `x` is a value (a local, a parameter, `self.repo`) collects every method
  of that name, from the project and from the standard library and dependencies, instead of
  stopping: Python `def` in a class, TypeScript methods and signatures (read from `.d.ts` outside
  the project), Go `func (r *T) Name(`. One candidate jumps, several open the picker. (#69)
- `d` on a word an import brings in from the project's own code, or on a member of that module,
  searches only the file or package the import names, and says so:
  `UserRepo: via import app/repos.py`, `Open: via import store/`. Python absolute and relative
  modules and packages, TypeScript relative files, `index` files and `tsconfig.json` `paths`,
  default, named and namespace imports, Go packages under the project's `go.mod`. An alias finds
  the name it imports (`UR` behind `from .repos import UserRepo as UR`). A module that
  only re-exports the word falls back to the search by name. An import of anything else goes to
  the standard library and the dependencies before the project's same-named declarations, so
  `json.dumps` behind `import json` no longer lands on a project `dumps`. (#73)
- `d` on `x.word` or `x.f.word` in Python, TypeScript and Go reads the type of the receiver from
  its declaration and looks for the member in that type and in what it extends or embeds:
  `self.repo`, `this.repo`, a Go receiver's field, a parameter or a local, annotated, constructed,
  handed a parameter, or assigned from a call whose function declares its return type (one hop).
  The status line names the link:
  `delete_user → UserRepository.delete_user (via self.repo: UserRepository)`,
  `via NewRepo() *UserRepository`. Declarations in scope that disagree, such as a variable
  shadowed inside a nested function, keep the search by name. (#83)
- `d` follows a chain such as `self.uow.users.delete_user` one field at a time, up to six names:
  each field is looked for in the type before it, in what that type extends, and in the Go structs
  it embeds. The status line lists the links,
  `via self.uow: UnitOfWork → users: UserRepository`. A chain that cannot be followed falls back
  to the search by name and says where it broke: `delete_user: by name, 2 declarations (chain
  broke at users)`. (#85)
- Twenty-three themes, taking the set to forty-seven. The families Vim and Neovim users run most
  and merl was missing: gruvbox, One Dark, GitHub, VS Code's Dark+ and Light+, Dracula with its
  light Alucard, Nord, Solarized, Oxocarbon, Sonokai, Material and nightfox itself. Plus the
  variants of the families already here: `tokyonight-night` and `tokyonight-storm`,
  `catppuccin-macchiato` and `catppuccin-frappe`. Every one is ported from its Neovim original;
  `docs/themes.md` says which themes ship and why.
- Forty-seven niche themes from the Vim and Neovim world, taking the set to ninety-four: iceberg,
  gotham, jellybeans, PaperColor, vague, miasma, lackluster, mellow, alabaster, bamboo, edge,
  moonfly, nightfly, srcery, e-ink, mellifluous, ayu, vesper, adwaita, neomodern, darkearth,
  token, koda, soviet, cendre, selenized, pencil and minischeme — each with its light variant
  where it has one.

## [0.5.0] - 2026-09-16

### Added

- `merl --review[=BRANCH] [--base REF]`: code review inside merl. The branch's diff against its
  base is a lens over the real files: added lines get a green mark, deleted lines are drawn in
  place as grey ghosts, and `d`, `u`, `s` work straight from the diff. The panel lists the
  branch's files with `M` / `A` / `D` and `+n −m`; `c` / `C` walk the hunks and cross into the
  next file; `[` comes back from an excursion. A deleted file opens read-only from the base.
  With `--review=BRANCH` merl fetches and switches to it; the base defaults to `origin/HEAD`, then
  `origin/master`, `origin/main`, `origin/develop`. (#60)
- Themes of your own: a `.tmTheme` in `~/.config/merl/themes/` is listed in `T` after the built-in
  ones and loads under its file name, and one named after a built-in replaces it, so a shipped
  theme can be copied and edited. A broken file names its path — at startup merl exits with code 1,
  and in `T` it says so and keeps the theme already on screen.

### Fixed

- Starting a selection inside a wrapped line no longer flashes its other rows back to the plain
  background: the cursor line keeps its highlight under a selection. Only a cursor line none of
  whose text is selected still drops it, so that line does not pass for selected (#48, #62).

## [0.4.0] - 2026-09-16

### Added

- Edit mode. Enter turns the cursor into a text cursor, Esc turns it back. Inside, merl is a plain
  editor with VS Code habits: letters insert, Enter splits the line and keeps its indentation, Tab
  indents the way the file already does, Backspace and Delete join lines, typing over a selection
  replaces it. There is no save step: edits reach the disk `autosave_delay_ms` (default 1000, a
  new config key) after the last keystroke and at once on Esc, on switching files and on quit;
  Ctrl+S saves now. A file changed on disk under unsaved edits is a conflict, not a reload: the
  status bar says so, Ctrl+S keeps the buffer and Ctrl+R takes the disk. Files keep their tabs,
  CRLF and trailing newline through a save; binary, non-UTF-8 and mixed-line-ending files stay
  read-only. Edits that could not be saved keep merl on the file: another file does not open over
  them and the first `q` is refused. The tutorial gets an editing lesson. (#11)
- Ctrl+Z / Ctrl+Y undo and redo. A run of keystrokes on one line is one step, as VS Code groups
  typing; a cursor move, a line split or join, or leaving edit mode starts a new one. The history
  is per open file. The tutorial gets an "Undo" lesson. (#11)
- In edit mode Ctrl+C and Ctrl+X copy and cut the selection, or the whole line without one, to
  the system clipboard through OSC 52, which the terminal forwards even over ssh. Paste is the
  terminal's own (bracketed paste), inserted while editing and ignored otherwise. (#11)
- Git marks in the gutter, as in VS Code: green for added lines, blue for changed ones, red under
  a line where lines were deleted, read from `git diff -U0` after every load, save and reload.
  (#11)
- `d` follows into the standard library and dependencies when the project has no definition:
  `sys.path` of the project's interpreter (a compiled module lands in its `.pyi` stub), the Rust
  sysroot and the crates in `Cargo.lock`, `GOROOT` and the modules of `go.mod`, `node_modules`.
  The file's imports say which module a qualified word (`json.load`, `fs::read`) comes from.
  Files outside the project open read-only. (#42)
- `d` and `D` in Rust: `fn`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`,
  `mod`, `macro_rules!` and `let` behind any `pub(..)` / `async` / `unsafe` / `extern` prefix;
  `impl` blocks are uses of the type, not definitions. (#25)
- `d` and `D` in TypeScript and JavaScript: `function`, `class`, `interface`, `type`, `enum`,
  `namespace`, `const` / `let` / `var` (so arrow functions assigned to a name), class and
  object-literal methods, behind any `export` / `default` / `declare` / `abstract` / `async`
  prefix. `.ts`, `.tsx`, `.js` and their module variants are searched together, so a `.tsx`
  component finds its types in `.ts`. (#31)
- `d` and `D` in Ruby. `d` finds `def`, `def self.name`, `class`, `module`, an assignment (a
  constant, an `@ivar`, a local), the names an `attr_accessor` / `attr_reader` / `attr_writer`
  line declares and an `alias` / `alias_method`, over every `.rb`, `.rake`, `.gemspec`,
  `.podspec`, `.rbi` and `.ru` file and `Rakefile`, `Gemfile`, `Vagrantfile` and friends; a
  trailing `?` or `!` is not part of the word, so `d` on `empty?` finds `def empty?`. `D` lists
  methods, classes and modules from a rule of its own, reading past the `self.` of a class method
  and the namespace of a `class Billing::Invoice`. Sorbet's `.rbi` files
  and `Dangerfile` now highlight as Ruby. (#17)
- `d` and `D` in Java and Kotlin, which are one kind: a `.kt` file finds the `.java` class it calls
  and the other way round, over every `.java`, `.kt` and `.kts` file. `d` finds Java's `class`,
  `interface`, `enum`, `record` and `@interface`, a method or constructor with a body, an abstract
  or interface method and a field, and Kotlin's `fun` (including an extension's receiver), `class`,
  `object`, `enum class`, `typealias` and `val` / `var`, behind annotations and the modifiers of
  either language. `D` lists them from rules of their own — Java's types and its methods, told from
  a call by the return type before the name, and Kotlin's `fun` (past an extension's receiver),
  types, `object`, `typealias` and `const val` — so the declaration pattern every other language
  shares is untouched and nothing is listed twice. (#17)
- `d` and `D` in SQL. `d` finds the `CREATE` of a table, view, index, function, procedure,
  trigger, type, schema, sequence, domain, extension, database, role or user — behind
  `OR REPLACE`, `TEMP`, `UNLOGGED`, `MATERIALIZED`, `UNIQUE` and `IF NOT EXISTS`, schema-qualified
  or quoted — and `WITH … AS (` common table expressions, across every `.sql`, `.psql`, `.pgsql`,
  `.mysql`, `.ddl` and `.dml` file; keywords ignore case. `D` lists the `CREATE`d objects under
  the name as written. `.psql`, `.pgsql` and `.mysql` now highlight as SQL. (#17)
- Highlighting for infrastructure files bat's set does not recognise by name: `.dockerignore` and
  `CODEOWNERS` (Git Ignore), `Containerfile` and `Dockerfile.*`, `*.jsonc`, systemd units and
  `.npmrc` (INI), `Procfile` and `yarn.lock` (YAML), `WORKSPACE` and `Tiltfile` (Starlark as
  Python). (#16)
- `d` and `D` in shell scripts. `d` finds `name()` and `function name`, an assignment behind
  `export` / `declare` / `local` / `readonly` / `typeset` (bare or `+=`) and an `alias`, across
  every `.sh`, `.bash`, `.zsh`, `.ksh` and shell dotfile (`.bashrc`, `.zshrc`, `.profile` and
  friends); `D` lists the functions, each once. (#17)
- `d` and `D` in Makefiles, Terraform, Dockerfiles and YAML. `d` finds Makefile targets and
  variables across every Makefile, the Terraform block behind `var.x` / `module.x` / `local.x` /
  `data.T.N` / `T.N` in the same directory, and Dockerfile stages, YAML anchors and block keys
  (compose services, CI jobs) in the same file. `D` lists targets, Terraform addresses, stages and
  anchors. A `-` is part of a word in these files. The tutorial gains a lesson on it. (#16)
- Twenty-one more themes, ported from their Neovim originals: Kanagawa Wave / Dragon / Lotus,
  Rosé Pine / Moon / Dawn, Everforest dark / light, Gruvbox Material dark / light, Catppuccin
  Mocha / Latte, Flexoki dark / light, Melange dark / light, Nordfox, Dawnfox, Dayfox, Tokyonight
  Day and Bluloco Light. `tools/port-theme.sh` ports a colorscheme in one command, and a test
  fails on a theme that paints code, YAML, Dockerfiles, Makefiles or TOML in too few colours. (#18)
- `T` picks a theme: the list repaints merl in the theme under the cursor as it moves, Enter keeps
  it and writes `theme` to `~/.config/merl/config.toml` (the rest of the file stays), Esc puts
  the old one back. Two new tutorial lessons teach it. (#18)

### Changed

- The file tree, `o` and project search include dotfiles (`.github/`, `.env`, `.dockerignore`);
  `.gitignore` still applies and `.git`, `.hg` and `.svn` are skipped. (#16)
- Dockerfiles use bat's `Dockerfile (with bash)` grammar: `RUN` lines are highlighted as shell and
  instruction arguments are no longer drawn in the default colour. (#16)
- All three themes colour the names infrastructure grammars emit: TOML keys and tables, INI
  sections, Terraform attributes, `.env` keys, Makefile and nginx variables, Dockerfile stages
  and image tags, YAML anchors and aliases. (#16)
- Long lines wrap between words instead of mid-word: a row ends after a space, and a word moves
  down whole. Only a word longer than the whole row (a URL, a call chain, a hash) is split where
  it stands, after `/ . , ; ) ] }` when it can. Rows after the first start under the text of the
  first, past the line's indentation and a list marker (`- `, `1. `), unless that takes more
  than half the pane.
- Up / Down, Shift+Up / Shift+Down, Ctrl+D / Ctrl+U and PgUp / PgDn move by screen rows, so a
  wrapped paragraph is walked row by row and half a screen is half of what the screen shows.
  Home / End, and Ctrl+Shift+Left / Right with them, stop at the start / end of the screen row
  first and go on to the line's on a second press, as in VS Code.

- A block cursor while navigating and a bar while typing and in every prompt, like vim, whatever
  the terminal's default shape is. (#41)
- `s>` project search looks for the text as typed, like `/`: `foo(` no longer fails as a bad
  pattern and `a.b` no longer matches `aXb`. There is no regex mode. (#52)
- `d` means definition: the whole-word fallback is gone, `u` lists uses. (#42)

### Fixed

- While text is selected the cursor line is highlighted in the gutter only, as in VS Code. A
  selection ending at the start of a line no longer looks like it takes that line, which Ctrl+C
  rightly leaves out. (#48)
- A `:` jump, a jump from a picker within the open file and a `/` search that moves the cursor
  drop the selection instead of stretching it to the new cursor; Esc on a search puts the
  selection back.
- A selection collapsed back onto its anchor selects nothing: Ctrl+C copies the line instead of
  an empty string, Backspace and Delete remove a char instead of marking the file changed, and a
  plain arrow moves.
- Ctrl+D / Ctrl+U and PgUp / PgDn only move the current history stop, so `[` after paging
  through a definition returns to the call site, not half a page back, and paging after `[` keeps
  the forward history. (#45)
- Quitting from the welcome screen or the `?` overlay left Ghostty and other xterm-like terminals
  without a cursor. (#46)
- Picking the open file in the `o` picker keeps the cursor instead of jumping to line 1. (#49)
- `d` finds a Kotlin `fun interface`. (#57)
- shokunin-light draws the selection in the theme's light blue instead of the cursor line colour.

## [0.3.0] - 2026-09-13

### Added

- `{` / `}` jump to the previous / next paragraph (blank line), like vim; in code that is the
  previous / next function without a parser. (#1)
- A welcome screen when merl starts without a file: the logo in the blues of the icon and the
  keys that work before a file is open. It shrinks to the keys alone, then to a one-line hint,
  when the pane is too small. (#9)
- `merl --tutor`: a vimtutor-style tutorial inside merl. Eighteen lessons over a sample Python
  project bundled in the binary, each one advancing when the key did what the lesson asked; the
  unpacked copy lives in a temporary directory and is removed on exit.

### Changed

- Find in file (`/`) matches literal text, as in VS Code: `migrator(` hits `Migrator()` instead
  of failing as an unclosed regex group. Smart case is unchanged; `s>` still takes a regex. (#5)
- A plain Left / Right on a selection collapses it to its start / end without moving further,
  as in VS Code; Up / Down still move from the cursor.
- The jump history now works like VS Code's: the current stop follows the cursor, so `[` goes
  back to where you were, not to where the last jump landed. A plain move farther than ten
  lines, Ctrl+D / Ctrl+U, `n` / `N` and find all add stops of their own; smaller moves update the
  current one. The history keeps the last fifty stops. Fixes #2.
- Enter in the tree on the file that is already open keeps the cursor where it is.

### Fixed

- A reload that shortened the file no longer crashes merl when a position taken before it is
  read back: Backspace to an empty `/` query, an arrow on a selection, `[` to an older stop.
  `[` onto a stop the file no longer reaches lands on the clamped line and keeps the forward
  history; a stop whose file is gone reports it and leaves the history alone.
- A line longer than 20 000 bytes (minified JS, single-line JSON) is wrapped the same way by
  the renderer and by the cursor arithmetic, so End no longer scrolls the line off the screen.
- Alt+letter over a picker, a prompt or the help only closes it; the letter is dropped instead
  of running a normal-mode binding (Alt+q used to quit).
- `u`, `d` and `s` search the open file even when the startup walk skipped it (hidden or
  ignored path opened by name).
- A result list cut at 5 000 hits says `(first 5000)` in the picker title.
- An invalid regex in `s>` is reported as `bad pattern`, not as `no results`.
- `?` scrolls with Up / Down when the terminal is too short for the whole list.
- The status bar says `no auto-reload` when no file watcher could be started, and `file gone`
  when the open file disappears from disk.
- Lines above a selection were painted with the selection background to the right edge.
- The usages, definitions and `s>` search pickers draw each hit with the syntax colours of the
  line it quotes, the same ones the code view shows after jumping there. Only the rows on
  screen are highlighted, so a picker over thousands of hits stays cheap. Fixes #7.
- A jump that went nowhere (`:` with the current line, for instance) erased the forward history.

## [0.2.0] - 2026-09-13

### Added

- Ctrl+D / Ctrl+U move half a screen, cursor and viewport together (vim/less style).
- A VS Code-style selection, painted with the theme's `selection` colour, that runs from where
  the first extending key was pressed to the cursor: Shift+Up / Shift+Down extend it by a line,
  Alt+Shift+Left / Right by a word, Ctrl+Shift+Left / Right to the start / end of the line (what
  Cmd+Shift+Left / Right does in VS Code, since Cmd never reaches a terminal program). Any other
  cursor move, Esc or opening a file clears it.

### Changed

- Shift+Up / Shift+Down no longer jump three lines; Ctrl+D / Ctrl+U cover fast movement.

### Fixed

- Go to definition finds annotated Python assignments (`NAME: Final[int] = ...`).
- Emptying the find query clears the previous highlights and returns to the anchor.

## [0.1.1] - 2026-09-12

### Fixed

- Symbol picker recognises `function` and declarations behind `export`, `pub`, `async` and similar prefixes (TypeScript, JavaScript, Rust `pub fn` were mostly missing).

## [0.1.0] - 2026-09-12

### Added

- Read-only file viewer with a line-number gutter and soft wrap at pane width.
- Syntax highlighting (syntect, bat syntax set) with tokyonight-moon, shokunin-light,
  shokunin-dark themes.
- CLI: `merl`, `merl DIR`, `merl FILE`, `merl FILE:LINE`, `--theme NAME`, `--version`.
- Project root resolution: the given directory, else the file's git toplevel, else its directory.
- Cursor movement: arrows, Shift+Up/Down (3 lines), Shift+Left/Right (word jump), PgUp/PgDn,
  Home/End, Ctrl+Home/Ctrl+End, with a sticky target column.
- Go to line via `:` or Ctrl+G; `q` and Ctrl+C quit, Esc closes the prompt.
- Status bar showing the path relative to the project root, the cursor position, and the focused pane.
- File tree in the left pane: `t` toggles it, Tab switches focus, Enter opens or expands,
  Left/Right collapse and expand; the tree reveals whatever file is opened.
- Fuzzy file picker (nucleo) on `o` / Ctrl+E, with matched characters highlighted.
- Find in file with smart-case regex: `/` or Ctrl+F searches as you type, `n` / `N` step through
  the matches with wraparound, and every match on screen is highlighted.
- Project search, go to definition (Python, Go), symbols and usages via ripgrep's library crates:
  `s`, `d` / F12, `D`, `u` / Shift+F12.
- Jump history: `[` and `]` walk back and forward through the positions a jump left behind.
- Config file `~/.config/merl/config.toml` with a `theme` key.
- Auto-reload: the open file is re-read when it changes on disk, keeping the cursor, the
  scroll position, the jump history and the find pattern.
- Help overlay on `?`, listing every binding; Esc in normal mode clears the find highlights.

[Unreleased]: https://github.com/Maksim-Burtsev/merl/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.5.0
[0.4.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.4.0
[0.3.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.3.0
[0.2.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.2.0
[0.1.1]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.1.1
[0.1.0]: https://github.com/Maksim-Burtsev/merl/releases/tag/v0.1.0
