# REPORT-c.md — `d` in C: why it misses (redis, oracle = clangd definition + declaration)

I read every WRONG, none, pick-miss and pick-hit row; the 68 `ok` rows were not audited.

## Corrected tally

| verdict | bench | artifacts | real | artifact share |
|---|---|---|---|---|
| ok | 68 | +1 moved in from WRONG | 69 (not audited) | — |
| pick-hit | 50 | 8: the cursor sits on a prototype or definition ("at a declaration") | 42 | 16% |
| WRONG | 14 | 1: c0039 `lua_settable` jumped to its real definition `deps/lua/src/lapi.c:645`; clangd only knew `lua.h:190` because `deps/` is not in compile_commands | 13 | 7% |
| pick-miss | 12 | 0 | 12 | 0% |
| none | 109 | 1: c0187, the cursor is on `strcpy`'s redeclaration in `fmacros.h:47` | 108 | 1% |

There are 133 real misses (WRONG + pick-miss + none):
- fields: 101 (76%)
- locals and parameters: 23 (17%)
- enum constants: 6
- a fallback or other-file macro: 3

## Causes, most harmful first

**1. A field has no rule — 101 misses (8 WRONG, 10 pick-miss, 83 none). Produces WRONG: yes, 8.**
- **What happens:** behind `->` or `.`, the word is searched as a top-level name (function, `#define`, global, struct tag, typedef) in the project, then in every system header.
- **Examples:**
  - `src/rax.c:617 memcpy(n->data,s,len);` jumps to `deps/tre/tests/retest.c:383 #define data wstr`. It should go to `src/rax.h:120 unsigned char data[];`.
  - `src/t_list.c:302 entry->entry.sz` jumps to `deps/jemalloc/test/unit/retained.c:7 static size_t sz;`.
  - `n->ip` offers SDK `struct ip`; `d->type` offers 403 libc++ `type` typedefs.
- **Fix:**
  - (a) Behind `->`/`.`, only a field is a candidate: nothing else, and no search outside the project.
  - (b) A field rule: a line directly inside a `struct`/`union` body. Scan up to the unmatched `{`; that line or the one above reads `struct`, `union` or `typedef struct`. It declares `T name;`, `T *a, *name;`, `T name[N];`, `T name:4;`, `T (*name)(…);`, or the `} name;` that closes a nested anonymous struct.
  - (c) Receiver typing, see issue 2.
- **Risk:** (a) none, it only refuses. (b) By name, 41 of 99 field cursors have exactly one field of that name and would jump right; 58 would be pickers, 40 of them with 5+ rows (`flags` has 53 fields, `type` 41, `count` 34).
- **Size:** (a) S, (b) M.

**2. A local or a parameter has no rule — 23 misses (4 WRONG, 19 none). Produces WRONG: yes, 4.**
- **What happens:** the name goes to the project by name, then to the system headers, where a struct tag or function of that name is waiting.
- **Examples:**
  - `src/cluster_legacy.c:1221 serverAssert(!link->node);` (`link` is the parameter on line 1220) jumps to SDK `unistd.h:476 int link(…)`.
  - `src/quicklist.c:1802 …(quicklist, node);` jumps to SDK `search.h:36 typedef struct node {`.
  - `group` goes to SDK `grp.h`, `struct group`.
- **Fix:**
  - (a) A word in value position (before `->`, `.` or `[`, or passed as an argument) is never a struct tag or typedef.
  - (b) A scope walk: from the cursor up to the function's column-zero header, take a declaration that starts a statement, or a parameter of the header, innermost block first. Skip strings and comments with the existing `code()`.
- **Risk:** the prototype `locals.py` got 17 of 19 `none` rows right and 3 of the 4 WRONG. Its errors were `a && b == c`, `x & FLAG` and a word inside a string read as declarations. So the rule must anchor at a statement start, reject `==` and keywords (`return`, `case`, `sizeof`), and skip strings.
- **Size:** (a) S, (b) M.

**3. A prototype and its definition make a two-row picker — 22 of the 42 real pick-hits. Produces WRONG: no.**
- **Example:** `src/t_string.c:419 addReplyErrorObject(…)` opens `networking.c:801` (definition) and `server.h:3402` (prototype).
- **Fix:** when every candidate declares the same function and exactly one has a body, jump to it, with the status `by name, 1 definition (+1 prototype)`. From the definition, the prototype stays one `d` away through the existing "at a declaration" offer.
- **Risk:** a namesake `static` in another `.c` file would count as a second definition; cause 5 removes those. clangd itself jumps to the definition.
- **Size:** S.

**4. An enum constant has no rule — 6 none. Produces WRONG: no.**
- **Example:** `src/db.c:3023 return KEY_VALID;` should go to `src/db.c:42 KEY_VALID = 0,`.
- **Fix:** `NAME[ = expr],` directly inside a block whose opener reads `enum [tag] {` or `typedef enum {`, found by the same brace scan as fix 1b; plus the one-line `enum X { A, B };`. An initializer list opens with `= {`, never `enum`. This answers the "same line" objection in `docs/navigation.md`.
- **Risk:** low.
- **Size:** S–M.

**5. Another file's local declarations and `#ifndef` fallbacks shadow the right answer — 3 misses (1 WRONG, 2 pick-miss), plus 12 wider pickers. Produces WRONG: yes, 1.**
- **Examples:**
  - `src/networking.c:4934 strcasecmp(…)` jumps to `deps/hiredis/win32.h:12 #define strcasecmp stricmp`, which sits under `#ifndef strcasecmp` in a Windows-only header. It should go to SDK `_strings.h:81`.
  - `INFINITY` offers the `#ifndef INFINITY` fallbacks of two `.c` files.
  - `assert` offers 5 rows; `dictDelete` also offers hiredis's `static` copy.
- **Fix:**
  - (a) A `#define`, a `static`, or a type declared in a `.c` file is visible only in that file (unless the current file `#include`s that `.c`).
  - (b) A `#define X` right under `#ifndef X` is a fallback: search outside first, and offer it only when nothing else declares `X`.
- **Risk:** low. The replay broke no `ok` row. The one fallback that was right (`REDISMODULE_ATTR`) still wins, because nothing outside declares it.
- **Size:** S.

**6. System roots are not narrowed, and a symlinked header counts twice — 4 wider pickers, and it fills the 10 field pick-misses. Produces WRONG: only through causes 1 and 2.**
- **What happens:** a `.c` file searches the whole SDK (libc++ `c++/v1`, `apr-1`, `tidy`, `sqlite3ext.h`) and all 187 packages under `/opt/homebrew/include`.
- **Examples:**
  - `pthread_equal` lists `pthread.h:352` twice.
  - `printf` adds gettext's `libintl.h`; `fileno` adds `tidy` and X11 headers.
- **Fix:**
  - (a) Canonicalize outside paths before dedup.
  - (b) A `.c` file never searches `c++/v1`.
  - (c) Narrow the outside search to headers reached by the file's `#include <…>`, followed by file name, transitively.
- **Risk:** (c) a header reached only through `-I` stays unread, so fall back to today's search when the walk finds nothing.
- **Size:** (a, b) S, (c) M.

**7. A line inside a macro body is read as a declaration — 1 miss (already counted in cause 2's WRONG). Produces WRONG: yes.**
- **Example:** `args->…` lands on SDK `apr-1/apr_hooks.h:119 typedef ret ns##_HOOK_##name##_t args; \`, a continuation line of a `#define`.
- **Fix:** a line following a `\`-continued `#define` line is macro body, not a declaration.
- **Related bug found:** a one-line `typedef struct X { int f; } X;` makes the typedef rule read `f` as a typedef name; its `[^;]*` must not cross a `{`.
- **Risk:** none.
- **Size:** S.

Not misses: 3 pickers are `#if`/`#else` variants of one macro (`USE_SETPROCTITLE`, `randomULong`, `atomicGetIncr`). They are honest.

## Pick-hit: why a picker, and what narrows it

The 42 real pick-hits split as:
- 22: a prototype and its definition (cause 3).
- 10: a namesake from another `.c` file or a vendored copy (cause 5).
- 5: a `typedef struct X {` / `} X;` pair offered as two rows, or a forward typedef beside a body in a `.c` file. A `typedef struct X {…} X;` in one file is one declaration; land on `} X;`, as clangd does.
- 4: system-header namesakes (cause 6).
- 3: honest `#if` variants.

**Replay result:** passing merl's candidate lists through causes 3, 5 and 6a plus the typedef pair turns 34 of the 42 into correct jumps, with no new WRONG. The 8 left are the 3 `#if` variants and 5 that need the include graph (`sdslen`, `dictEntry`, `fileno`, `memmove`, `printf`).

## Latency

Median 113 ms, p90 118 ms. There are two regimes:
- About 30 ms when the project declares the word.
- About 115 ms when it does not: every field, local and enum constant (the 109 `none` rows) greps all system roots before answering "no definition".

Four rows took over 300 ms:
- c0000 `addr`, 613 ms: the first probe, on a cold cache.
- c0123 `value` 569 ms, c0053 `end` 531 ms, c0140 `type` 396 ms: fields turned into pickers of 133 to 403 libc++ namesakes.

Fix 1a removes all four, and the 115 ms of every field lookup.

## Top improvements

**1. C/C++: `x->field` and `x.field` land on a struct field, never on a function, macro, global or type.**
- Today this gives 8 WRONG, 10 junk pickers, and costs about 115 ms per field.
- Step one: refuse everything but fields behind `->`/`.`, and skip the outside search.
- Step two: the field rule from cause 1b. By name it lands 41 of 99 field cursors; the rest are pickers until issue 2.
- Repro:
  ```c
  /* buf.h */    struct buf_s { char *data; int len; };
  /* compat.c */ #define data wstr
                 static int len = 0;
  /* use.c */    int f(struct buf_s *b) { return b->len + (b->data != 0); }
  ```
  | cursor | today | should |
  |---|---|---|
  | `data` in `b->data` | `compat.c:1` | `buf.h` |
  | `len` in `b->len` | `compat.c:2` | `buf.h` |

**2. C: prove the struct of a receiver from its declaration (`client *c` → `c->argv`).**
- Read `x` from the function's parameters and statement-start locals, or else from a column-zero global declared once.
- Resolve the type through `typedef struct TAG {…} T;`, `typedef struct TAG T;` plus `struct TAG {`, or `struct T {`. A body in another `.c` file is not visible.
- The field's declared type is the next link of a chain.
- The 150-line prototype `typed.py` resolved 85 of 101 member cursors to the oracle's field, with 0 wrong. Unresolved: `union {…} bs;` members, declarator lists that wrap, and `struct ldbState {…} ldb;`. A cast or call in front of the member stays by name.
- Repro:
  ```c
  /* server.h */ typedef struct client {
                     int flags;
                 } client;
  /* other.h */  struct job {
                     int flags;
                 };
                 int flags;
  /* use.c */    #include "server.h"
                 void f(client *c) { c->flags = 0; }
  ```
  Cursor on `flags` in `c->flags`: today it jumps to `other.h:4` (the global); it should go to `server.h:2`, `via c: client`.

**3. C/C++: a local or a parameter is its own declaration, and a value is never a struct tag.**
- Today: 4 jumps into system headers, and 19 "no definition" for variables declared a few lines up.
- Walk up to the column-zero function header. Anchor declarations at a statement start and skip strings.
- Until that lands, a word in value position must at least never land on a type.
- Repro:
  ```c
  struct node_s { int v; };
  int g(struct node_s *link) { return link->v; }
  ```
  Cursor on `link` in `link->v`: today it jumps to SDK `unistd.h:476` in 484 ms; it should go to the parameter.

**4. C/C++: a call jumps to its one definition, not to a picker with its prototype.**
- 22 of 42 real pick-hits are exactly {definition, prototype}.
- Add the file-local rule and the `#ifndef` fallback rule from cause 5: the replay then turns 34 of 42 into correct jumps with no new WRONG, and fixes the `strcasecmp` WRONG.
- Repro: `util.h: int add(int a, int b);`, `util.c: int add(int a, int b) { return a + b; }`, `main.c: … add(1, 2)`. Cursor on `add`: today a picker of `util.c:2` and `util.h:1`; it should jump to `util.c:2`.

**5. C/C++: an enum constant is a line inside an `enum {` body.**
- 6 none here and 3 in leveldb. The brace scan tells an enum body from an initializer list; also cover the one-line `enum X { A, B };`.
- Repro: `enum color { RED, GREEN }; int main(void) { return GREEN; }`. Cursor on `GREEN`: today "no definition" after 109 ms; it should go to line 1.

---

# REPORT-cpp.md — `d` in C++: why it misses (leveldb, oracle = clangd definition + declaration)

I read every WRONG, none, pick-miss and pick-hit row; the 43 `ok` rows were not audited.

**Oracle caveat:** `build/compile_commands.json` holds only the 39 library files. clangd answered tests, benchmarks and `helpers/` with fallback flags. That is why gtest macros get `namespace leveldb {`, and why for `RandomKey` and `AddBoundaryInputs` the oracle gives only the declaration its translation unit sees.

## Corrected tally

| verdict | bench | artifacts | real | artifact share |
|---|---|---|---|---|
| ok | 43 | — | 43 (not audited) | — |
| pick-hit | 104 | 10: the cursor is on a declaration (7 "at a declaration", plus `TableBuilder::Rep {`, `Status Delete(…) override;`, `void Clear();`) | 94 | 10% |
| WRONG | 6 | 0 | 6 | 0% |
| pick-miss | 41 | 3: the cursor is on an out-of-line definition (`TableBuilder::FileSize`, `VersionSet::VersionSet`, `VersionSet::Recover`) | 38 | 7% |
| none | 45 | 5: 3 gtest/benchmark macros whose oracle is `namespace … {`; `HAVE_ZSTD`, defined only in the gitignored generated `build/include/port/port_config.h`; `NewNode` with the cursor on its definition | 40 | 11% |

There are 84 real misses:
- data members: 38 (45%)
- receiver type unknown or a `std::` type: 15
- pure virtual or in-class declarations: 15
- locals and parameters: 11
- enum constants: 3
- two system-header shapes: 2

## Causes, most harmful first

**1. A data member has no rule — 38 misses (3 WRONG, 7 pick-miss, 28 none). Produces WRONG: yes, 3.**
- **Examples:**
  - `db/db_impl.cc:721 CompactRange(m->level, …)` jumps to `version_set.h:325 int level() const`, a method of `Compaction`. It should go to `db_impl.h:81 int level;`.
  - `db_bench.cc:944 thread->rand.Uniform(…)` jumps to SDK `int rand(void)` instead of `db_bench.cc:368 Random rand;`.
- **Shapes:** `x->f` / `x.f` 20; a constructor's initializer list (`: mu(mutex)`, `rnd_(0xdeadbeef)`) 5; a bare member used inside a method (`filename_`, `iter_`, `dbname_`) 3.
- **Fix:**
  - (a) A member not followed by `(` is a field: no method, function, type or macro, and no outside search.
  - (b) A field rule: a line directly inside a class body, such as `T name_;`, `T name = …;` or `T name GUARDED_BY(mu_);`.
  - (c) In an initializer list, `name(` is a field of the class being constructed. A bare name inside a method of `X` is `X`'s field first.
- **Risk:** (a) none. (b) By name, 15 of 39 field cursors have exactly one field of that name; the rest are pickers of 2 to 5+ rows.
- **Size:** (a) S, (b, c) M.

**2. A type name competes with its forward declarations, its constructors, and nested types defined through it — about 40 real pick-hits. Produces WRONG: no.**
- **Examples:**
  - `Status s;` opens a picker of `class Status {`, `inline Status::Status(const Status&)` and `Status::Status(Code, …)`; 16 real pick-hits are this `Status` picker alone.
  - `Slice` shows 5 `class Slice;` lines beside its body.
  - `Table` also lists `struct Table::Rep {`.
- **Side effect:** it switches off the existing path rule. `Status::Corruption(…)` sees 3 declarations of `Status` and falls back to a by-name picker of every `Corruption` (cpp0018, and `WriteBatch::Handler`).
- **Fix:**
  - (a) A namespace-level `class X;` yields to the one body of `X`.
  - (b) `X::X(`, `X<…>::X(` and an in-class `explicit X(` count only when the cursor's word is called, as in `X(`.
  - (c) `struct X::Rep {` and `SkipList<K,C>::Node {` declare `Rep` and `Node`, not `X`, and must be recognised as such (`struct DBImpl::Writer {` is not recognised at all today).
- **Risk:** the replay turns 38 of 94 pick-hits into correct jumps (42 together with cause 5), with no new WRONG. A nested forward declaration (`struct Writer;` inside `DBImpl`) must not yield to a body in another namespace.
- **Size:** S.

**3. The receiver's type is unknown — 15 misses (2 WRONG, 13 pick-miss), plus about 25 real pick-hits. Produces WRONG: yes, 2.**
- **What happens:** a member call is matched against every method of that name in the project, and with a single namesake it jumps, even on a `std::` object.
- **Examples:**
  - `db/version_set.cc:532 inputs->clear()` on a `std::vector<…>*` jumps to `Slice::clear`. `db/dumpfile.cc:173 r.clear()` on a `std::string` does the same.
  - `tables_.size()` offers `Slice`/`Block`/`BlockHandle::size`.
  - `scratch->append(…)` offers 148 candidates from libc++ `filesystem` and simdjson. The real one is in the extensionless `<string>`, which merl never reads.
- **Fix:** find the receiver's class from a parameter or local (`const Slice& key`), from a field of the enclosing class (found through `Class::method(` or the class body around an inline method), or from `this->`. Then look in that class's body, its out-of-line `Class::name(` definitions, and its bases (`: public Base`). A `std::` or outside type gets no project candidate.
- **Risk:** the prototype `typed_cpp.py` resolved 48 of 97 member cursors: 31 jump to the oracle's target, 16 give narrower pickers, 1 is a picker without the target, 0 are wrong, and 5 more are proven to be `std::`. Templates, `auto` and chained calls stay by name.
- **Size:** M–L.

**4. A pure virtual or an in-class declaration has no rule — 15 pick-miss. Produces WRONG: no.**
- **Example:** `table/table_test.cc:636 iter->Valid()` offers 11 overrides but not `iterator.h:35 virtual bool Valid() const = 0;`. The same happens for `Compare`, `Seek`, `Next`, `Flush`, `Sync`, `status` and `NewIterator`.
- **Also:** on the out-of-line definition `VersionSet::Recover(`, the "1 other by name" offered is `DBImpl::Recover`, from a different class.
- **Fix:**
  - (a) Inside a class body, a line containing `virtual … name(…) … ;` or `… name(…) … override;` is a declaration; a call carries neither word.
  - (b) A plain `T name(args);` directly inside a class body. The brace scan tells it from `Slice key(k, n);` in a function.
  - (c) On `X::name(`, the counterpart is `name(` in `X`'s body.
- **Risk:** (a) none. (b) Low, given the opener check.
- **Size:** (a) S, (b, c) M.

**5. Another translation unit's local declarations are offered, and prototype + definition stays a picker — about 11 real pick-hits. Produces WRONG: no.**
- **Examples:** `FLAGS_num` offers the `static int FLAGS_num` of 3 benchmarks; `Between` offers 2 tests' statics; `FileState` offers 2 files; `ParseFileName`, `Hash` and `CurrentFileName` are definition + prototype pairs.
- **Fix:** same as in C: a `static`, a `#define` or a type of another `.cc` file is no candidate, and one definition beats its prototypes.
- **Risk:** the replay's two "WRONG" (`RandomKey`, `AddBoundaryInputs`) jump to the real definitions; only the incomplete oracle lacks them.
- **Size:** S.

**6. A local or a parameter has no rule — 11 misses (1 WRONG, 2 pick-miss, 8 none). Produces WRONG: yes, 1.**
- **Example:** `memenv_test.cc:133 …&result…` (`Slice result;` on line 98) jumps to `result()` in simdjson's header under `/opt/homebrew/Cellar`. `delete db;` lists `db()` methods.
- **Fix:** the C scope walk, plus: in a function body, `T name(args);` and `T name{…};` are locals too.
- **Size:** M.

**7. An enum constant has no rule — 3 none. Produces WRONG: no.**
- **Example:** `table_test.cc:433 case DB_TEST:` should go to `:374 enum TestType { …, DB_TEST };`.
- **Fix:** the C enum rule, including the one-line form.
- **Size:** S–M.

**8. Outside roots are not narrowed, and libc++ is only half read — 2 misses (1 pick-miss, 1 none), plus the WRONG in cause 6.**
- **What happens:** all of `/opt/homebrew/include` is searched (simdjson's 186k-line header; `errno` from `libwebsockets.h`; `fprintf` from gettext).
- **Missed shapes:**
  - `class _LIBCPP_EXPORTED_FROM_ABI _LIBCPP_CAPABILITY("mutex") mutex {` is missed, because `c_mods!(macros)` takes `[A-Z]…` and `__…` macros but not `_LIBCPP_…`.
  - `__sized_by_or_null(__size) malloc(` is missed, so `std::malloc` gets none.
- **Fix:**
  - (a) Accept `_[A-Z]\w*` macros in the type and function rules.
  - (b) Read the extensionless headers under `c++/v1/`.
  - (c) Narrow outside roots by the file's `#include` graph.
- **Size:** (a, b) S, (c) M.

Not misses: 5 honest `#if` pickers (`LEVELDB_EXPORT` ×2, `THREAD_ANNOTATION_ATTRIBUTE__`, SDK `assert` ×2).

## Pick-hit: why a picker, and what narrows it

The 94 real pick-hits split as:
- About 40: type names (forward declarations, constructors, `X::Rep {`). Fixed by cause 2.
- About 11: other translation units' statics, or definition + prototype. Fixed by cause 5.
- About 30: member calls and bare calls matched by name. Fixed by cause 3, plus "the enclosing class first" for a bare call inside a method (`key()`, `size()`, `CompactMemTable()`, `dbfull()`).
- 11: outside namesakes (`swap`, `atomic`, `push_back`, `c_str`, `sqlite3_*` from `sqlite3ext.h`, `errno`). Fixed by the include graph.
- 5: honest `#if` variants.

**Replay result:** the S rules alone turn 42 pick-hits into correct jumps with no new WRONG (44 counting the two oracle gaps).

## Latency

Median 5 ms, p90 92 ms; a search outside the project costs about 90 ms. Five rows took over 300 ms:
- `append` ×4, about 625 ms each: 148 candidates from libc++ `filesystem` and simdjson.
- `type` in the constructor initializer `type(t)`, 370 ms: 403 libc++ typedefs.

Fix 1a (a non-called member never searches outside) removes the `type` case. Receiver typing plus the include graph removes the `append` cases.

## Top improvements

**1. C++: a type name lands on its class, not on its forward declarations and constructors.**
- About 40 of 94 real pick-hits.
- A namesake constructor or forward declaration also switches off `Type::member`.
- The replay converts 38 into correct jumps with 0 new WRONG.
- Repro:
  ```cpp
  // status.h  class Status { public: Status(); static Status Corruption(const char* m) { return Status(); } };
  // status.cc #include "status.h"
  //           Status::Status() {}
  // env.h     class Status;
  // iter.h    struct Reporter { virtual void Corruption(int bytes) {} };
  // use.cc    Status s;  …  return Status::Corruption("bad");
  ```
  | cursor | today | should |
  |---|---|---|
  | `Status` in `Status s;` | picker of `env.h:1`, `status.cc:2`, `status.h:1` | `status.h:1` |
  | `Corruption` in `Status::Corruption` | picker of `Reporter::Corruption` and `Status::Corruption` | `via Status` (it already jumps there once `env.h` and `status.cc` are gone) |

**2. C/C++: a member that isn't called is a field, never a method, function or type; plus a field rule.**
- Today: 3 WRONG and 7 junk pickers.
- Refuse the other kinds first, then add the field rule and the initializer-list rule `: name(…)`.
- Repro:
  ```cpp
  class Compaction { public: int level() const { return level_; } int level_; };
  struct Manual { int level; };
  void f(Manual* m) { int x = m->level; }
  ```
  Cursor on `level` in `m->level`: today it jumps to `Compaction::level()`; it should go to `Manual::level`.

**3. C++: pure virtual and `override` declarations in a class body are declarations.**
- 15 pick-miss. With the rule, `it->Valid()` offers `Iterator::Valid`, and a second `d` there lists the implementations through the existing flow.
- Repro:
  ```cpp
  class Iterator { public: virtual bool Valid() const = 0; };
  class A : public Iterator { public: bool Valid() const override { return true; } };
  class B : public Iterator { public: bool Valid() const override { return false; } };
  void f(Iterator* it) { it->Valid(); }
  ```
  Cursor on `Valid`: today a picker of lines 2 and 3 only; it should reach line 1, `via it: Iterator`, or at least offer it.

**4. C++: find the receiver's class from its declaration, and never land a `std::` receiver in the project.**
- Fixes 2 WRONG, turns 13 junk pickers honest, and narrows about 25 method pickers. The prototype resolved 48 of 97 member cursors with 0 wrong.
- Repro:
  ```cpp
  #include <vector>
  class Compaction { public: void clear() {} };
  void f(std::vector<int>* v) { v->clear(); }
  ```
  Cursor on `clear`: today it jumps to `Compaction::clear`; it should go outside, or say "no definition".

**5. C/C++: another `.cc` file's statics don't count, and one definition beats its prototypes.**
- About 11 pickers: `FLAGS_num` ×3, `Between` ×2, `FileState` ×2, `ParseFileName`, `Hash`, `CurrentFileName`.
- Shared with the C report's issue 4, with the same `add()` repro.
