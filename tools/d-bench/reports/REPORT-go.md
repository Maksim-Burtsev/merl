# Go `d` misses: causes from the bench run (caddy, oracle gopls)

The run has 270 cursors. 41 are `on-decl` and not scored, which leaves 229: ok 143 (62%), pick-hit 27, pick-miss 30, none 20, WRONG 9.

## Tally of the rows read

I read every non-ok row: all 9 WRONG, all 20 none, all 30 pick-miss and all 27 pick-hit. Of the pick-hits, 21 were opened one by one; the other 6 repeat a shape already checked (`t.Errorf` ×3, `newRequest` ×3).

| verdict | bench | real | artifact / debatable |
|---|---|---|---|
| WRONG | 9 | 9 | 0 |
| none | 20 | 20 | 0 |
| pick-miss | 30 | 28 | 2 debatable (go0028 `ResponseWriter:`, go0243 `Reader:`): the key names an embedded field. gopls lands on the embedded-field line, and merl's picker held the embedded interface type. |
| pick-hit | 27 | 26 | 1 debatable (go0136 `syscall.Errno` in a `_windows.go` file): gopls answers for the file's own GOOS. merl, on a darwin host, offers all 5 platform variants on purpose. |
| ok (among rows read) | 0 | 0 | none of the rows read turned out to be right |

- **Artifact share:** about 0% for WRONG and none, about 7% for pick-miss, about 4% for pick-hit.
- **Sampler quirk:** the `type` shape is mostly composite-literal keys. 26 of the 31 `type`-shaped misses are keys.
- **Check for fix 1:** all 57 `ok` rows that resolved a bare word `by name` landed in the cursor's own package.

**The lead is confirmed.** 29 of the 86 non-ok rows are composite-literal keys (26 have the `type` shape, 3 the `name` shape): 4 WRONG, 9 none and 16 pick-miss.
- **Where the WRONG ones went:** one to a namesake type in another package (go0013), two to namesake methods (go0112 `Dispenser.Line`, go0147 `testAddr.Network` in another package's test file), and one to a function in a dependency (go0248, smallstep `func DNS()`).
- **What the pickers held:** namesake functions and types outside the project, and `:=` locals from other files.

A prototype built only from line rules (`work-go/keyproto.py`) resolves 27 of those 29 rows to the oracle's exact line. The 2 it misses are keys inside anonymous structs.

## Causes, most harmful first

A WRONG jump usually has two causes: a gap in the rules (the trigger) and the fallback that turned the gap into a jump. For 7 of the 9 WRONG rows that fallback is cause 1.

| # | cause | count (W/none/p-miss/p-hit) | WRONG? | example | fix sketch | risk | size |
|---|---|---|---|---|---|---|---|
| 1 | **A bare Go identifier is searched everywhere.** The search includes receiver methods (`func (r T) Name(`), `x :=` locals in every file, other packages, and then GOROOT plus every go.mod module. | 9 pick-hit as the main cause; it is the fallback behind 7 of 9 WRONG | yes: go0013, 0066, 0112, 0117, 0147, 0232, 0248 | go0142 `listeners.go:331 if IsUnixNetwork(network) {` opens a picker with the method `NetworkAddress.IsUnixNetwork` (:209) beside the function at :299. go0066 `replacer.go:272 sb.WriteString(empty)` jumps to `sb := serverBlock{…}` at `caddyconfig/httpcaddyfile/directives_test.go:87`; the answer is `replacer.go:183`. | Follow Go's own scope rule. After the scope walk, call `package_declarations(here, word, None)`, which already exists and already leaves methods (`T.m`) out. If it finds nothing, try dot imports, then say "no definition". Bare words stop using the method pattern, cross-file `:=` and the outside search. The on-declaration namesake path stays. | Low. All 57 bare `by name` ok rows were in the same package. Guard: 179 of the 8,948 `Ident:` sites in caddy share a name with a top-level declaration of their own package, so a key-shaped word must not take a package answer (ship with #2 or refuse). `package x_test` files share the directory; read the package clause. | S |
| 2 | **Composite-literal keys `Field:` have no rule.** They are looked up as bare words. | 29 (4/9/16/0) | yes | go0147 `listeners.go:369 Network: network,` inside `return NetworkAddress{` jumps to `testAddr.Network()` at `modules/caddytls/matchers_test.go:315`; the answer is `listeners.go:47`. go0112 `lexer_test.go:200 {Line: 1, …}` inside `expected: []Token{` jumps to `Dispenser.Line()` at `dispenser.go:252`; the answer is `lexer.go:43`. | 1. For `Word:` (not `:=`, not after `.`, not on a `case`/`default` or label line), scan back over code, skipping strings, raw strings and comments, to the unclosed bracket.<br>2. If that bracket is `{`, read the type in front of it: `T{`, `&T{`, `pkg.T{`, `T[A]{`.<br>3. An elided `{` (one after `{`, `,` or `key:`) takes its parent's element type: `[]T`, `[N]T` or `[]*T` give `T`; a map value gives `V`. Repeat as needed. `}{` is an anonymous struct whose body is right above.<br>4. Look the field up with the existing typed lookup, as for `T{}.Word`, embedded structs included.<br>Refuse:<br>- map literals: their keys are values, so the normal lookup applies;<br>- a `{` after `)` or a keyword: that is a block;<br>- a named type that is not a struct;<br>- a type that cannot be found: answer "no definition", never by name. | Low. The prototype got 27 of 29 bench rows exactly. Over all 8,948 `Ident:` sites in caddy it resolves 4,207; spot-checks were all right. It refuses the rest: 4,304 anonymous structs (see #6) and 415 types it cannot read (function-local types, named slice types, `type X Y`). | M |
| 3 | **Members of `const (`, `var (` and `type (` blocks are not declarations.** The Go patterns are `^type`, `^(var\|const)` and `:=` only. | 11 (3/8/0/0) | yes: go0117, 0169, 0232 | go0117 `browsetplcontext.go:234 l.Sort == sortByName` jumps to `func sortByName` in `golang.org/x/net@v0.58.0/http2/hpack/gen.go:161`; the answer is `browsetplcontext.go:397`, inside `const (`. go0169 `celmatcher.go:654 ast.CallKind` jumps to `type CallKind int` in `golang.org/x/tools/internal/typesinternal`; the answer is the iota line `CallKind` at cel-go `common/ast/expr.go:29`. The none rows are `http.StatusInternalServerError`, `http.MethodGet`, `time.Hour`, `time.Minute`, `caddyhttp.DefaultHTTPSPort`, `MatchPath{…}` and `MatchHost{…}`. | Add a pattern, `^\t(?:\w+\s*,\s*)*W\b(?:\s*,\s*\w+)*(?:\s\|=\|$)`, that counts only when the line sits `directly_inside` a column-0 `const (`, `var (` or `type (`. Terraform's `locals {` already works this way through `def_block`. `qualified` must return None for these lines, so they read as top-level. | Low. Under gofmt a struct field is never directly inside `type (`; it sits inside `Name struct {`. | S |
| 4 | **An import-bound qualifier falls back to a by-name search elsewhere.** When the package lacks the word, `pkg.X` is searched by name across the project and the module cache. | the fallback in go0169; repro in issue 1 | yes | In the repro, `shop.MaxItems` (a `const (` member of package `shop`) jumps to `func MaxItems` in package `other`. | Go has no re-exports. `pkg.X` behind an import whose directory exists is declared in that directory or nowhere, so the answer should be "no definition for X in pkg/". | None. Cgo's `C.x` has no directory, so it is unchanged. | S |
| 5 | **Receiver types declared outside the project are never proven.** Fields outside the project are not collected by name either. | 25 (0/0/12/13) | no | go0094 `admin.go:1090 r.URL.Path` (`r *http.Request`) opens a picker of 10 project `URL` fields; the answer is `net/http/request.go:131`. go0096 `wg.Add(1)` (`var wg sync.WaitGroup`) opens a picker of 1,875 `Add` in 521 ms; the answer is `sync/waitgroup.go:77`. | Let `declaration()` resolve `pkg.T` inside the external package directory the import binds. This is the resolution `via import` already uses: `GOROOT/src/<path>` or `<module>@<ver>/<sub>`. `Typed` then holds an outside path, and `members_of`, `field_of` and the embedded walk read it. That covers `testing.T` → `common.Errorf`, interface method lines (`ctx.Err`, `w.Write`, `ln.Close`) and one-hop results (`httptest.NewServer`). Refuse per-platform declarations the host does not build (`host_built` already). | Low: a Go package is one directory. It is also faster: one directory is read instead of about 400 MB. | M |
| 6 | **Anonymous structs in table-driven tests are not read.** | 6 (1/1/2/2), plus 2 keys | yes: go0106 | go0106 `builtins_test.go:406 if !tc.errorFunc(err)`, under `for i, tc := range []struct {` (:341), jumps to `RetryConfig.errorFunc()` in `google.golang.org/api/internal/gensupport/retry.go:113`; the answer is `:343`. go0109 `encode_test.go:381 test.name` (`testCases := []struct {`) opens a picker of 33. | There are two gaps.<br>(a) The walk starts the inline header `for … range []struct {` … `}{` … `} {` at `}{`. `tc` stays unbound, and the word falls to the project-wide rules; that is the WRONG jump. Fix: a closer whose opener line ends in `struct {` or `}{` belongs to the header above it.<br>(b) An element of `[]struct {`, inline or in `x := []struct {`, is the struct body at that line. `field_bindings(text, decl, name)` already reads a body starting from a line. | Low: the body is right there, nothing is guessed. | M |
| 7 | **A label in column 0 ends the function for the scope walk.** gofmt outdents labels one level, so the walk takes `scan:` for a header and leaves the function; the locals and parameters above it are lost. caddy has 10 such labels in 9 files. | 2 (1/1/0/0) | yes: go0066 | go0242 `tlsapp.go:1025 aps[j]`, below `outer:` at :1020, gives "no definition". `aps` is a parameter at :969. go0066 (see #1) sits below `scan:` at `replacer.go:195`. | `block_bindings` skips a Go line that is only `Ident:`. | None | S |
| 8 | **A Go raw string that ends in `\` never closes in `literal_lines`.** It applies the JS template escape (`b[i-1] != '\\'`) to Go. | 1 (0/1/0/0); it hides 1,590 lines in 3 files | no | go0246 `replacer_test.go:440 getEOL()` gives "no definition", even on its own declaration at :453, because `` `{\\\}\\\\` `` at :161–162 flips the rest of the file. It also hides 1,129 lines of `modules/caddyhttp/matchers.go` after `` `\` `` at :660: `normalizeWindowsEscapedPath` (:699) gives "no definition" from its call at :476. | Keep the escape for TS/JS only; in Go a backtick always closes. | None | S |
| 9 | **A same-block redeclaration `n, err :=` counts as a second declaration.** | 1 pick-hit | no | go0124 `commandfuncs.go:675 err = …` opens a picker of {663 `input, _, _, err := …`, 648 `err := …`}; the answer is 648. | Go allows `:=` in a block only with a new name on the left, so the first declaration in the innermost block is the declaration. | None | S |
| 10 | **Rare shapes, where by name is fine.** | 2 as the main cause (go0079, go0136), plus 2 also counted under #5 (go0005, go0234) | no | Four shapes, one row each:<br>- a parallel assignment `a, b := f(), g()` with a func-literal callee: go0079;<br>- the open file's own GOOS: go0136, debatable;<br>- an index expression ends the chain: go0005 `writerPools[k].Get()`;<br>- a method call in the middle of a chain: go0234 `je.Encoder.Clone().EncodeEntry`. | No new rules; each stays by name. go0005 and go0234 also need #5. | – | – |

Nothing in the sample needs real type inference. The rows in #10 stay by name, which is acceptable for how rare they are.

With causes 1–9 fixed, about 80 of the 86 non-ok rows would become right answers. Every fix is a rule over lines.

## Pick-hit rows (27): why a picker, and what would narrow it

- **13 rows, receiver type outside the project (#5).** `t.Errorf` ×4, `wg.Add`, `upv.refs.Add`, `s.timer.Reset`, `ctx.Err`, `m.logger.Check`, `w.Write`, `ln.Close`, `server.Serve`, `writerPools[k].Get`. Proving `*http.Request`, `sync.WaitGroup`, `testing.T` (through its embedded `common`) and the interfaces `context.Context` and `http.ResponseWriter` in their package directory turns each into a jump.
- **9 rows, bare word not kept to its package (#1).**
  - `newRequest` ×4: a `newRequest := func` closure in `fastcgi_test.go`, which is another package, is offered.
  - `isCELStringLiteral`, `CA` and `Handler` are each declared in two or more packages.
  - `IsUnixNetwork` has a method namesake.
  - `Error` has `Error()` methods among its candidates.
  - `package_declarations` gives one answer each.
- **2 rows, anonymous struct (#6):** `tc.match.Validate` and `tc.match.MatchWithError`, both reported as "chain broke at tc".
- **1 row, redeclaration (#9):** `err`.
- **1 row, parallel assignment:** go0079 `handlerToUse.Response.HeaderOps`. It holds two candidates, which is fine.
- **1 row, platform:** go0136. Preferring the declaration built for the open file's own `_GOOS` suffix would narrow it; this is optional.

## Latency

- **Distribution:** 198 rows took under 50 ms and 72 took 300 ms or more. Every row of 300 ms or more searched outside the project: `GOROOT/src` plus the 169 go.mod roots, 11,864 non-test `.go` files and about 396 MB, grepped on every such `d`. The median was about 370 ms.
- **Common method names cost more:**
  - `Get` (4,562 candidates): 2.8–5 s.
  - `String` (4,086): 0.6–1.6 s.
  - `Add` and `Reset` (1,875 and 2,224): 0.5–0.7 s warm.
  - The file list is walked once per session, but the grep over it and the build of a picker with thousands of rows are paid on every `d`.
- **Cold start:** the first `d` of a run is slower (go0000 took 2.3 s cold and 0.66 s warm). The timings are noisy with the load average near 4.
- **How the fixes change it:** of the 72 slow rows, 21 keys and 8 bare words would not leave the package or the literal's type under #1 and #2. The 43 dotted rows would read a single package directory under #5.

## Top improvements (issue drafts)

### 1. Go: an unqualified name is declared in its own package; `d` stops searching methods, other packages and the module cache for it

- A bare identifier in Go is one of four things: a local, a top-level name of its package (every file in the directory, and `_test.go` files from a test), a builtin, or a name from a dot import.
- Today `d` greps every `.go` file with the receiver-method pattern and with `x :=`, then GOROOT and all go.mod modules. That fallback produced 7 of the 9 WRONG jumps in the caddy bench, and it left 9 pickers that should be jumps.
- **Proposed rule:** after the scope walk, answer with `package_declarations(here, word, None)`, then dot imports, then "no definition".
- Apply the same idea to `pkg.X` behind an import whose directory exists: no by-name fallback, since Go has no re-exports.
- **Guard:** a key-shaped `Word:` inside `{…}` takes no package answer until issue 2 lands.

```go
// shop/types.go                                   // other/other.go
package shop                                       package other
const (                                            func MaxItems() int { return 3 }
	MaxItems = 10
)
type NetworkAddress struct{ Network string }
func (na NetworkAddress) IsUnix() bool { return false }
func IsUnix(n string) bool            { return n == "unix" }
func check(n string) bool             { return IsUnix(n) } // d on IsUnix: picker {NetworkAddress.IsUnix, IsUnix}; want IsUnix
// app/main.go:  _ = shop.MaxItems   // d: "MaxItems: by name, 1 match" → other.MaxItems (WRONG)
```

### 2. Go: `d` on a composite-literal key `Field:` lands on the field of the literal's type

- This covers 29 of the 86 caddy misses: 4 WRONG (namesake methods and types), 9 none and 16 pickers without the field.
- **Proposed rule:**
  - Scan back over code to the `{` that opens the literal, and read the type in front of it (`T`, `&T`, `pkg.T`, `T[A]`).
  - For an elided `{{`, climb to the parent's element type (`[]T`, `[N]T`, `[]*T`, the map value).
  - Look the field up with the existing typed path, as for `T{}.Field`, embedded structs included.
- **Refuse:** map literals, blocks (`{` after `)` or a keyword), non-struct types and unknown types. Refusing means "no definition", never a by-name jump.
- **Evidence:** a prototype built from these rules alone (`work-go/keyproto.py`) resolves 27 of 29 bench rows exactly.

```go
type Order struct {                    func build(addr string) Order {
	Address string                         return Order{
	Items   []Item                             Address: addr,        // d → type Address (by name, 1 match): WRONG
}                                              Items: []Item{
type Item struct{ Name string }                    {Name: "a"},      // d on Name → Address.Name: WRONG
type Address struct{ Street string }           },
func (a Address) Name() string { … }       }
                                       }
```

### 3. Go: members of `const (`, `var (` and `type (` blocks are declarations

- This covers 11 misses, 3 of them WRONG: `http.StatusOK`, `time.Hour`, `http.MethodGet`, `caddyhttp.DefaultHTTPSPort` and `ast.CallKind` (an iota line) all give "no definition" or a namesake. Type-block members such as `MatchPath{…}` and `[]Token{` do the same.
- **Proposed rule:** a line `\tName …`, `\tA, Name = …` or a bare `\tName` counts only when it sits directly inside a column-0 `const (`, `var (` or `type (`. This is the `def_block` / `directly_inside` rule Terraform already uses for `locals {`.
- `qualified` must read these lines as top-level.

```go
const (
	sortByName = "name"   // `return s == sortByName` → d: no definition (repro); in caddy a WRONG jump into x/net
)
type (
	Order struct{ Address string } // `Order{…}` → picker of GOROOT's compress/lzw Order and another namesake
)
var (
	DefaultTimeout = 5 * time.Second // → no definition;  `time.Hour` → no definition
)
```

### 4. Go: prove receiver types declared outside the project (stdlib and go.mod dependencies)

- This covers 25 rows, 13 pick-hit and 12 pick-miss, and 43 of the 72 slow rows.
- **Proposed rule:** `declaration()` resolves `pkg.T` in the external package directory its import binds, which `via import` already computes. `members_of`, `field_of` and the embedded walk then read that directory.
- This covers four shapes:
  - struct fields: `r.URL`, `resp.Body`;
  - methods through an embedded struct: `t.Errorf` via `common`;
  - interface method lines: `ctx.Err`, `w.Write`;
  - one-hop call results: `srv := httptest.NewServer(…)`.
- Per-platform files stay subject to `host_built`.

```go
func handle(r *http.Request, t *testing.T) string {
	var wg sync.WaitGroup
	wg.Add(1)          // d: picker "Add: by name, 179 declarations"; want sync/waitgroup.go
	t.Errorf("x")      // d: picker of 2; want testing.go:1354 (common.Errorf via embedded common)
	return r.URL.Path  // d: picker of 2 traceviewer methods, field missing; want net/http/request.go (URL *url.URL)
}
```

### 5. Go scope walk: table-test structs and column-0 labels

- **Table tests:**
  - `for _, tc := range []struct {…}{…} {` is read from its `}{` line, so `tc` is unbound, and `tc.name` falls to the project-wide rules. In caddy that gave a WRONG jump into google.golang.org/api (go0106).
  - For `tests := []struct {…}`, the loop variable is bound but its struct has no name, so the chain breaks at `tc`.
  - Fix: read the header back to the `for` line, and type an element of `[]struct {` as the struct body at that line, using `field_bindings` from a line.
  - Anonymous structs make up 48% of the `Ident:` key sites in caddy.
- **Labels:** gofmt puts a label (`scan:`, `outer:`) in column 0, and the walk takes it for the end of the function.
  - Everything declared above the label is lost for cursors below it.
  - Fix: skip a line that is only `Ident:`.

```go
func labelled(input string) string {    func table() {
	var sb strings.Builder                  for _, tc := range []struct {
	sb.Grow(len(input))                         name  string
scan:                                           check func(error) bool
	for i := 0; i < len(input); i++ {        }{
		if input[i] == 'x' {                     {name: "a"},     // d on name → other package's `name := "x"`: WRONG
			continue scan                    } {
		}                                        _ = tc.name      // d → picker of 14 by name, field missing
		sb.WriteString("y")  // d on sb → other package's `sb := 1`: WRONG
	}                                        }
	return sb.String()                   }
}
```

**Also (S, a one-line fix):** `literal_lines` lets `\` escape a Go raw string's closing backtick. `` `\` `` and `` `{\\\}\\\\` `` never close, and every declaration after them in the file "declares nothing". In caddy that hides 1,129 lines of `modules/caddyhttp/matchers.go`; for example, `normalizeWindowsEscapedPath` gives "no definition" from its call at :476. Make the escape TS/JS-only.

## Files (in `d-bench/work-go/`)

- `repro/`: the minimal Go module every repro above was run on.
- `keyproto.py`: the composite-key prototype.
- `keysweep.py`: runs the prototype over every `Ident:` site in caddy.
- `lit.py`: a port of `literal_lines` that counts the raw-string damage.
- `run.sh` and `mk.py`: the bench runner and a builder for cursor rows.
