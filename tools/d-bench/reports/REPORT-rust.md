# `d` in Rust: cause analysis (ripgrep, oracle rust-analyzer)


The bench ran 300 cursors in `proj/ripgrep`, with merl at `origin/master` (acff182). I read all 22 WRONG, all 19 none and all 38 pick-miss rows (8 of the pick-miss rows are the same `#[derive(` case). I checked all 79 pick-hit rows for the oracle line and its file, and read about 35 of them in full. Every repro below was run through `zz_bench` in `work-rust/repro/`. Paths are relative to `proj/ripgrep`.

**The lead:**
- **Confirmed for `unwrap`, `clone` and `hash`.** `member_patterns(Kind::Rust)` is `None`, so `x.m()` greps the top-level patterns over the project. It jumps on a single hit and looks outside only when the project has nothing.
- **Confirmed for `self.field`.** Rust has no field rule. That gives 4 WRONG, and the same gap causes 4 none and 14 pick-miss.
- **`.open` is a different cause.** It is `File::open`, a `::` path whose `use std::fs::File` is ignored (cause 4), not a member.

## Tally (rows read)

| Verdict | Bench | Artifacts | Real |
|---|---|---|---|
| ok | 81 | Not read (74 are `by name, 1 match`, 7 are proven) | – |
| pick-hit | 79 | 2 → pick-miss (scorer artifacts, see below) | 77 |
| WRONG | 22 | 0 | 22 |
| pick-miss | 38 | 3 → pick-hit (see below) | 35 |
| none | 19 | 0 | 19 |

**pick-hit artifacts.** For outside targets the scorer compares only the file:
- 0091: any `…/src/lib.rs` counts as the oracle's `…/src/lib.rs`.
- 0274: `struct Capture` counts as the variant `Capture` in the same file.

**pick-miss artifacts:**
- 0004 `std::collections` and 0280 `std::fs`: merl lists the `pub mod …;` line in std's `lib.rs`, while the oracle points at the module's own file.
- 0137: merl lists `ToString::to_string`, while rust-analyzer redirects to the `Display` impl's `fmt`.

**none.** Three `to_string` rows have the same debatable oracle, but merl showed nothing there, so they stay real.

**Corrected totals:**
- pick-hit 80, WRONG 22, pick-miss 37, none 19.
- Artifact share: WRONG 0 %, none 0 %, pick-miss 8 %, pick-hit 3 %.

**Unscored rows hide three more wrong jumps:**
- 0014: `Choose` inside a string jumps to `rand`'s `struct Choose`.
- 0049: `#[cfg(test)]` jumps to `SearcherTester::test`.
- 0205: on the variant `PCRE2(…)`, `d` jumps to `struct PCRE2;`.

## Causes, most harmful first

Counts are WRONG/none/pick-miss.

| # | Cause | Count | WRONG? | Example | Fix sketch | Risk | Size |
|---|---|---|---|---|---|---|---|
| 1 | A method on a value of unknown type is looked up by name in the project first, and outside only if the project has none | 23 (11/6/6) | **Yes, 11** (`unwrap` ×5, `clone` ×5, `hash`) | `json.rs:1052` `.unwrap()` → a private method of an unrelated crate | Rust member lines, then project + std + dependencies, minus what the cursor cannot reach | No new WRONG; some lucky jumps become pickers | M |
| 2 | Field access `x.f` has no rule | 22 (4/4/14) | **Yes, 4** | `stats.rs:38` `self.searches` → the `fn searches` accessor | No `(` after the word means a field; read `self`'s type from the enclosing `impl` | Low | S + M |
| 3 | Words inside `#[…]` are read as project names | 12 (4/0/8), +1 unscored WRONG | **Yes, 4** | `#[derive(Debug)]` → the project's `struct Debug;` | Derive and attribute macros only; compiler built-ins get no search at all | None | S |
| 4 | A `use` of std, a dependency or a workspace crate does not steer the lookup | 5 (2/0/3), +5 pick-hits | **Yes, 2** | `gitignore.rs:624` `File::open` → the project's `MmapChoice::open` | Map each crate to a folder and search that crate before the project | Low | M |
| 5 | Enum variants have no rule | 13 (1/8/4), +1 unscored WRONG | **Yes, 1** | `CaseMode::Sensitive` finds nothing | Read variant lines inside an `enum` body; honour `use Enum::*` | Low | S |
| 6 | Strings that span lines are read as code | 1 unscored WRONG, about 30 unscored cursors on prose | Yes (unscored) | `Choose` inside a string → `rand` | Let a string run to its closing quote across lines | Low | S |
| 7 | Locals and parameters have no scope | 2 (0/0/2), +3 pick-hits, noise in member pickers | No | `= config;` → 8 unrelated rows | Scope `let` and parameters to the enclosing block | Medium | M |
| 8 | Outside module paths match loosely | 2 artifacts, +2 pick-hits, +1 none | No | `std::fs` → 33 rows | Go to the exact module file first | None | S |
| 9 | The file's own item is not preferred for a bare name | 0 misses; 30 of 79 pick-hits could jump | No | `mkdirp(…)` → a picker of 3 | A bare name the file declares is that item | Low | S |

### 1. A method on a value of unknown type

Rust has no member rule, so the project is searched by name first, with `let`, `mod` and top-level `fn` lines included. Outside is searched only when the project has nothing. A value name in front of the dot (`hostname.to_string()`) is read as a module path, so nothing is found outside either.

**Examples:**
- `crates/printer/src/json.rs:1052` `got.lines().nth(1).unwrap()` goes to `GramQuery::unwrap` at `crates/index/src/literal.rs:57`. That method is private and lives in `grep-index`, which `grep-printer` does not depend on. It should go to `core/src/option.rs:1011`.
- `crates/globset/src/glob.rs:97` `self.glob.hash(state)` goes to the enclosing `fn hash` on line 96.

**Fix sketch:**
- Add Rust member lines: an indented `fn w` that `qualified()` names `Type::w`.
- Collect project + std + dependencies, as Python, TS and Go already do.
- Drop what cannot be reached:
  - a method without `pub` in an inherent `impl` outside its file;
  - a `pub(crate)` method outside its crate;
  - crates missing from the cursor crate's `Cargo.toml` dependency graph.
- If every candidate is a trait's method or sits in an `impl` of that trait, offer the trait's declaration first.

**Risk and size:** no new WRONG. A few of the 8 lucky `member` ok rows become pickers (for example `arg`, which std's `Command::arg` also declares). Size M.

### 2. Field access `x.f` (no call) has no rule

The method, `let` or `mod` with the same name answers instead.

**Examples:**
- `crates/printer/src/stats.rs:38` `self.searches` goes to `fn searches` on line 37; it should go to the field on line 15.
- `crates/searcher/src/line_buffer.rs:145` `self.config.capacity = capacity` goes to the method `LineBufferBuilder::capacity`; it should go to the field on line 88.

**Fix sketch:**
- `x.w(` or `x.w::<` means methods only.
- Any other `x.w` means field lines only: `^\s+(pub(..)\s+)?w\s*:` under a `struct … {` header.
- On `self.w`, the `impl … for T` line above the cursor names `T`. Look the field up in `struct T` and chain through the field types. That covers 11 of the 22 rows.
- A typed parameter or a `T::default()` construction covers 3 more.

**Risk and size:** low risk, because the header check keeps struct literals and wrapped parameter lists out. The field rule is S; the `self` chain is M.

### 3. Words inside `#[…]` are read as project names

**Examples:**
- `crates/core/flags/defs.rs:2306` `#[derive(Debug)]` goes to `struct Debug;` at defs.rs:1533. It should go to `pub macro Debug` in `core/src/fmt/mod.rs:1093`.
- `#[derive(` offers `syn`'s `mod derive` twice, after a 127 ms grep each time. It should go to `core/src/macros/mod.rs:1769`.
- `#[test]` goes to `SearcherTester::test`.

**Fix sketch:**
- Inside `derive(…)`: look for `pub macro W`, which needs `macro` added to the Rust keywords, or `#[proc_macro_derive(W` in a dependency.
- For the attribute's own name: look for `pub macro W` in core, or a `#[proc_macro_attribute]` fn in a dependency.
- For words in `cfg(…)` and for compiler built-ins such as `allow`, `inline` and `repr`: answer "no definition" without a grep.

**Risk and size:** no risk, size S.

### 4. A `use` does not steer the lookup

Rust's `imported_definitions` returns `Some([])` (definition.rs:677), so the project is searched by name before anything else.

**Examples:**
- `crates/ignore/src/gitignore.rs:624` `File::open(path)` goes to `MmapChoice::open` (searcher/mmap.rs:65). It should go to `std/src/fs.rs:569`.
- `crates/printer/src/standard.rs:1706`: `io` in `-> io::Result<()>` goes to `CommandError::io`.

**Fix sketch:**
- Map the first segment of the path to a place to search:
  - `crate`, `self` and `super` go to `rust_use_files`, including for a bare imported word;
  - a workspace crate is found by the `name` in its `Cargo.toml`;
  - `std`, `core` and `alloc` go to the sysroot;
  - a crate listed in Cargo.lock goes to its registry folder.
- Search that crate first, filtering `Type::w` with `qualified()`, and search the project by name only after it.

**Risk and size:** low, since this only narrows the search. A re-export under another crate name still falls back to the search by name. Size M.

### 5. Enum variants have no rule

**Examples:**
- `crates/core/flags/defs.rs:763` `CaseMode::Sensitive` finds nothing; it should go to lowargs.rs:305.
- `crates/globset/src/lib.rs:691`: `Regex(ref s) =>`, under the function's `use self::GlobSetMatchStrategy::*;`, goes to regex-automata's `struct Regex` through the file's own `use`. It should go to lib.rs:661.

**Fix sketch:**
- A variant line is `^\s+(#[..]\s*)*W\s*([({,=]|$)` under an `enum … {` header.
- `qualified()` already names such a line `E::W`, so the existing `pathed` step proves `E::W`.
- A bare `W` with a `use …E::*` in scope is `E`'s variant. The function's own `use` counts first, then the file's.

**Risk and size:** low risk thanks to the header check. Size S.

### 6. Strings that span lines are read as code

`literal_lines` ends a `"` string at the end of its line.

**Example:** `tests/regression.rs:406`: `Choose`, inside a string that continues with `\`, goes to `STD/vendor/rand-0.9.2/src/distr/slice.rs:59`. Prose words give pickers such as `of: by name, 5 declarations` and `to: 16`, each after a 128 ms search outside the project.

**Fix sketch:**
- A `"` string runs to the next unescaped `"`, across lines.
- Also read `r#"…"#`, counting the `#` signs, and `b"…"`.
- A cursor inside such a string gets "no definition" without a search.

**Risk and size:** low risk, because lifetimes are already handled at syntax.rs:198. Size S.

### 7. Locals and parameters have no scope

`bindings()` covers only Python, TS and Go, and `let w` is matched across the whole project.

**Examples:**
- `crates/printer/src/standard.rs:184`: the right-hand `config` in `= config;` offers 8 unrelated rows, including `mod config;` and another file's `let config`. The right answer is the parameter on line 182.
- `crates/index/src/literal.rs:245` `or(…)`: the binding `let (or, and) = …` on line 208 is never read.

**Fix sketch:**
- Rust block bindings, as Go has: `let`, `let (…)`, `if let`, closure parameters `|w|`, `for w in`, and the function's parameters, including ones wrapped one per line.
- Take the nearest block above the cursor.
- Drop `let` from the project-wide patterns.

**Risk and size:** medium risk, because shadowing such as `let x = x.trim()` is common. A pattern the rules cannot read must hide the outer bindings rather than guess. Size M.

### 8. Outside module paths match loosely

The module path is matched as a subsequence of the file path (`in_module`), not as the file the path spells.

**Examples:**
- `crates/ignore/src/dir.rs:1125`, on `fs` in `std::fs::…`, offers 33 rows: the `mod fs` of every `std::os::*`, the first being a `let fs` in std's own tests.
- `Instant::now` offers 25 rows, with `std/src/time.rs:285` in 21st place.

**Fix sketch:**
- `k::a::b` goes first to `<k>/src/a/b.rs` or `a/b/mod.rs`, then to that folder, then to the whole crate.
- Skip the sysroot's `vendor/`, `*tests/` and `benches/` folders.
- A primitive-type qualifier (`usize::MAX`) goes to core, searched by name.

**Risk and size:** no risk, size S.

### 9. The file's own item is not preferred for a bare name

This causes no misses, but 30 of the 79 pick-hits could be proven jumps.

**Examples:**
- `crates/ignore/src/dir.rs:1391` `mkdirp(…)` opens a picker of 3 test helpers; the file's own is at line 1129.
- `crates/pcre2/src/matcher.rs:464` `RegexMatcherBuilder::new()` offers 90 `new`s, because the type is declared both here and in another crate and the path is refused.

**Fix sketch:**
- A bare word that the file declares as an item in the cursor's module (column 0, or the same inline `mod`) is that item. Rust sees another file's items only through a `use` or a path.
- At definition.rs:281–292, set `home = [here]` when this file declares the owner type.
- Related bug (0247): with the cursor on the call in `let globs = globs(…)`, merl thinks it is standing on a declaration. Compare the declared name's column, not just the line.

**Risk and size:** low risk, size S.

## pick-hit rows: why a picker, and what would narrow it

- **30 rows: a bare name the cursor's file declares once.**
  - Examples: `mkdirp` ×6, `printer_contents` ×4, `extract` ×2, `exact` ×2, `tmpdir` ×2, `SHERLOCK`, `Context`, `Config`, `Message`, `RegexMatcherBuilder` ×2, `ROOT`.
  - Plus `RegexMatcherBuilder::new` (0037).
  - Cause 9 fixes these.
- **35 rows: the receiver's type decides.** About 14 would become proven jumps with a Rust receiver-type step (cause 2, extended to typed parameters, `T::new()` and `-> T` return types):
  - the typed closure parameters of ripgrep's `rgtest!(… |dir: Dir, mut cmd: TestCommand|)` cover `create` ×5 and `stdout` ×4;
  - `let td = tmpdir()` and `let ov = ov(&[])` go through `fn … -> T` (`path` ×3, `matched` ×2).

  Builder chains (`.build(`), `.unwrap().to_string()` and `td.path().join` need real type inference; a picker is the right answer there.
- **5 rows: the name is bound by a `use`.** Cause 4 fixes these:
  - `use crate::non_matching::non_matching_bytes` (0039);
  - `use grep_matcher::Match` (0068, 0256);
  - `RegexMatcher::new` (0209);
  - `pub use crate::matcher::RegexMatcherBuilder` (0277).
- **3 rows: local closures and bindings.** Cause 7 fixes these:
  - `err` (0257);
  - `exts` (0245);
  - `analysis` (0040), which is shadowed by another function's `let mut analysis`.
- **2 rows: loose outside path** (cause 8): `Instant::now` (0047) and `fs::remove_file` (0149).
- **2 rows: trait methods.** In `flag.doc_long()` (92 rows) and `name_negated` (36 rows), every candidate is `trait Flag`'s method or sits in an `impl Flag for X`, so the answer is `Flag::doc_long` (cause 1).
- **2 rows: scorer artifacts** (see the tally).

## Latency

**Distribution:** median 6.4 ms, 90th percentile 128 ms, maximum 918 ms. Only one row is over 300 ms: rust0003.

**The 918 ms row is the first lookup outside the project in the session.** `external_files(Rust)` walks the sysroot (about 2,900 `.rs` files once the `library/library` copy is skipped) and the 63 Cargo.lock crates (about 1,850 `.rs` files). Re-run on its own, that cursor took 929 ms cold and 128 ms warm.

**61 of the 300 rows (20 %) take 124–143 ms.** Each of them greps every outside file by name:
- 42 are unscored, mostly prose inside strings that span lines (`of`, `the`, `Sherlock`, man-page `.sp`), plus the attributes `cfg`, `allow` and `deny`.
- 8 are `#[derive(`.
- Some are variants (`Literal`, `Suffix`, `PathWithoutMatch`).
- Some are methods on a call result, such as `td.path().join` (49 rows).

**What would bring it down:**
- Causes 3, 5 and 6 remove most of these searches.
- Causes 4 and 8 narrow the rest to a single crate.
- Skip the sysroot folders no `use` can reach: `vendor/` (875 files, std's own copies of rand, memchr and others), `coretests` and `alloctests` (230), and `benches`. That removes about 1,100 files from every search, along with noise rows like `STD/vendor/rand … Choose` and `STD/coretests … to_string`.
- Walk the outside roots in the background when the first `.rs` file opens, so the first `d` does not pay about 0.8 s.

## Top improvements (issues)

### 1. Rust: `x.method()` on a value of unknown type lists the methods of every crate it can reach, never a lone project namesake

- **Today:** `member_patterns(Kind::Rust)` is `None` (defs.rs:408). So `x.m()` greps `fn`, `let` and `mod` over the project, jumps on one hit, and goes outside only when the project has none. With a value name in front, `external_definitions` reads it as a module and finds nothing.
- **Member lines:** give Rust an indented `fn w` that `qualified()` names `Type::w`. The value path then collects project + std + dependencies, as it does for Python, TS and Go.
- **Drop what the cursor cannot reach,** reading only lines and `Cargo.toml` files:
  - a method without `pub` in an inherent `impl T {` outside its file;
  - a `pub(crate)` method outside its crate;
  - any workspace crate the cursor's crate does not depend on, directly or indirectly.
- **Trait methods:** if every candidate is a trait's method or sits in an `impl` of that trait, offer the trait's declaration first. `flag.doc_long()` goes from 92 rows to `Flag::doc_long`.
- **Sample:** 11 WRONG, 6 none, 6 pick-miss.

```rust
// other/src/lib.rs  (crate `other`, not a dependency)
pub struct Query;
impl Query {
    fn unwrap(self) -> Query { self }
}
// src/lib.rs
pub fn first(v: Option<u32>) -> u32 {
    v.unwrap()            // d on unwrap
}
```

- **Now:** `unwrap → Query::unwrap (by name, 1 match)`.
- **Wanted:** a picker led by core's `Option::unwrap` and `Result::unwrap`, without `Query::unwrap`.
- The same happens with `impl Clone for Error { fn clone … }` plus `n.clone()` on a `&String`: `clone → Error::clone`.

### 2. Rust: `x.field` is a struct field, and `self` is the type of the `impl` around it

- **Today:** Rust has no field rule. `self.searches` lands on the accessor `fn searches` right above it, and `self.config` offers `mod config;` and other files' `let config`.
- **Tell the two apart by syntax:** `x.w(` or `x.w::<` is a method call; any other `x.w` is a field. A field line is `^\s+(?:pub(?:\([^)]*\))?\s+)?w\s*:` whose enclosing line is a `struct … {` header, not a function signature and not a literal.
- **`self.w` and `self.a.w`:**
  - The `impl … for T` / `impl T` line above the cursor names `T`; the `IMPL` regex in `qualified()` already reads it.
  - Look `w` up in `struct T`. Its type is the next link in the chain, after stripping `&`, `mut`, lifetimes and `Box`/`Rc`/`Arc`.
  - A generic type ends the chain.
- **Typed parameters** are read the same way: `low: &LowArgs`, and the `|dir: Dir, cmd: TestCommand|` of ripgrep's `rgtest!`.
- **Sample:** 4 WRONG, 4 none, 14 pick-miss; 11 of these 22 rows are `self.f` or `self.a.f`.

```rust
pub struct Stats {
    searches: u64,
}
impl Stats {
    pub fn searches(&self) -> u64 {
        self.searches     // d on searches
    }
}
```

- **Now:** `searches → Stats::searches (by name, 1 match)`, landing on the function.
- **Wanted:** the field, `(via self: Stats)`.
- Likewise `self.config.capacity = capacity;` lands on the method `Builder::capacity` instead of `Config`'s field.

### 3. Rust: a word inside `#[…]` is an attribute or a derive macro, never a project item

- **Today:**
  - `#[derive(Debug)]` jumps to ripgrep's flag type `struct Debug;`.
  - `#[test]` and `#[cfg(test)]` jump to `SearcherTester::test`.
  - `#[derive(` offers `syn`'s `mod derive` after a 127 ms grep of every dependency.
- **For a word between `#[` (or `#![`) and its `]`:**
  - inside `derive(…)`: look for `pub macro W` (core's built-in derives; add `macro` to the Rust keywords) or a dependency's `#[proc_macro_derive(W`;
  - the attribute's own name: look for `pub macro W` (`test` and `derive` live in core) or a dependency's `#[proc_macro_attribute] pub fn W`;
  - words in `cfg(…)` and the compiler's own attributes (`allow`, `inline`, `repr`): answer "no definition" immediately, without a grep.
- **Checked:** the sysroot has exactly one `pub macro` each for `derive`, `Debug`, `test`, `Clone` and `PartialEq`, and each is the oracle's target.
- **Sample:** 4 WRONG and 8 pick-miss, all of which would become ok.

```rust
struct Debug;
#[derive(Debug)]      // d on Debug
struct Column;
```

- **Now:** `Debug: by name, 1 match` → `struct Debug;`.
- **Wanted:** `pub macro Debug` in `core/src/fmt/mod.rs`.

### 4. Rust: a `use` names the crate to look in; search that crate before the project by name

- **Today:** `imported_definitions` returns `Some(vec![])` for Rust (definition.rs:677), and navigation.md says "Rust still looks in the project first". As a result:
  - `File::open` behind `use std::fs::File` lands on the project's only `fn open`;
  - `io` in `io::Result` lands on `CommandError::io`;
  - `io::Error` offers the project's `Error` types.
- **Map the first segment of a path to a root:**
  - `crate`, `self` and `super` → `rust_use_files`. Today that runs only for `Type::w` with two or more declarations; it should also run for a bare imported word such as `use crate::non_matching::non_matching_bytes`.
  - A workspace crate → found by the `name` in its `Cargo.toml`, with `-` turned into `_`.
  - `std`, `core` and `alloc` → `library/<k>/src` in the sysroot.
  - A crate listed in Cargo.lock → its registry folder.
- **Inside that crate:**
  - Look first in the file the path spells (`std::fs` → `std/src/fs.rs` or `fs/mod.rs`, not `os/*/fs.rs`).
  - Then look in that folder, then in the whole crate, to catch re-exports.
  - Filter `Type::w` with `qualified()`.
  - Search the project by name only when all of that is empty.
- **Same-file owner:** for `Type::w` with `Type` declared both in this file and in another crate, set `home = [here]` instead of refusing the path (definition.rs:281).
- **Sample:** 2 WRONG, 3 pick-miss, plus 5 pick-hits that would become jumps.

```rust
use std::fs::File;
pub struct Mmap;
impl Mmap { pub fn open(&self) {} }
pub fn read(p: &str) {
    let _ = File::open(p);    // d on open
}
```

- **Now:** `open → Mmap::open (by name, 1 match)`.
- **Wanted:** `open → File::open (via import std::fs)`.
- The same repro with `use std::io;` and a project `enum ErrorKind` jumps to that enum from `io::ErrorKind::Other`.

### 5. Rust: enum variants, both `Enum::Variant` and a bare variant behind `use Enum::*`

- **Today:** no rule reads a variant line. So:
  - `CaseMode::Sensitive` is "no definition" (8 rows like this);
  - `FlagLookup::Match` offers a picker of four unrelated `Match` types;
  - `Regex(ref s) =>` under `use self::GlobSetMatchStrategy::*;` jumps to `regex_automata::meta::Regex` through the file's `use`.
- **Variant line:** `^\s+(?:#\[[^\]]*\]\s*)*W\s*(?:[({,=]|$)` whose enclosing line is an `enum … {` header. `qualified()` already names it `E::W`, so the existing `pathed` step proves it and reports it `via E`.
- **Bare variant:** a bare `W` with a `use …::E::*;` in scope is `E`'s variant. The enclosing function's `use` counts first, then column 0, ahead of a file-level `use x::W`.
- **Sample:** 1 WRONG, 8 none, 4 pick-miss, plus 1 unscored WRONG.

```rust
pub struct Regex;
pub enum Strategy {
    Literal(String),
    Regex(u32),
}
impl Strategy {
    fn is_regex(&self) -> bool {
        use self::Strategy::*;
        match *self {
            Regex(_) => true,     // d on Regex
            Literal(_) => false,
        }
    }
}
```

- **Now:** `Regex: by name, 1 match` → `pub struct Regex;`.
- **Wanted:** `Regex → Strategy::Regex`.
- With `pub enum Mode { Auto, Never }` written one variant per line, `Mode::Auto` gives `no definition for Auto`.

### Next, and cheap: cause 9

Let the file's own item win for a bare name. That turns 30 of the 79 pick-hits into proven jumps.

- `mkdirp(…)` at `crates/ignore/src/dir.rs:1391` opens a picker of 3 test helpers, although the file's own is at line 1129.
- `fn globs()` plus `let globs = globs();`, with the cursor on the call, says `at a declaration, 1 other by name`.
