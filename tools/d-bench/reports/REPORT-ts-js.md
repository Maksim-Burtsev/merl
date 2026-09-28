# REPORT-ts: `d` in TypeScript (outline, compared against tsserver)

The project has **no `node_modules`**. I read all 9 WRONG, 29 none, 15 pick-miss and 13 pick-hit rows (66). Causes 1–5, 7, 8, 11, 13 and 14 were reproduced in scratch projects.

## 1. Corrected tally

| verdict | bench | real | artifact | what the artifacts are | artifact share |
|---|---|---|---|---|---|
| WRONG | 9 | 6 | 3 | ts0158, ts0163: merl lands on `const {` / `function X({`, and the name sits 2 and 12 lines further down in the same pattern. ts0222: the oracle picks `export default observer(AccountMenu)`; merl picks the component itself, which is better | 33% |
| none | 29 | 11 | 18 | No `node_modules`: 9 names from TypeScript's own lib (`includes`, `stringify`, `Omit`, `Record`, `Reflect`…) and 9 imports of packages that aren't installed, where tsserver answers with the import line | 62% |
| pick-miss | 15 | 5 | 10 | 8 because there's no `node_modules` (3 lib, 5 uninstalled packages); 2 with the cursor on a destructuring binding, which is itself a declaration | 67% |
| pick-hit | 13 | 13 | 0 | ts0246 is the 3 overloads of one function, a correct picker | 0% |

After removing artifacts: **ok +3, WRONG 6, none 11, pick-miss 5, pick-hit 13**. 28 rows can't be scored. With `node_modules` installed, ts0134 would have been a picker rather than a jump.

## 2. Causes, most harmful first

| # | cause | count | WRONG? | example (merl → should) | fix sketch | risk | size |
|---|---|---|---|---|---|---|---|
| 1 | A class or interface whose type parameters prettier wrapped is dropped as "a wrapped call". `show_definitions` (D:435) drops any hit ending in `<` unless `declares_wrapped_generic` sees `>(…) {`. A type header closes with `> {` or `> extends … {` instead, so every Sequelize model (`class User extends ParanoidModel<`) and `interface APIContext<` is thrown away | 8 (3 WRONG, 4 none, 1 pick-miss) | **yes**: only the client-side namesake is left, and it gets the jump | `server/routes/api/collections/collections.ts:518` `UserMembership.count(options)` → `app/models/UserMembership.ts:10` "by name, 1 match"; should be `server/models/UserMembership.ts:71` `class UserMembership extends IdModel<`. `plugins/linear/server/api/linear.ts:40` `ctx: APIContext<…>` → none; should be `server/types.ts:92` | Keep a `<`-ending hit that `search::declares_type` matches, or that has `extends`/`implements` before the `<`. Refuse (still drop) only a non-declaration line whose closer at its indent starts with `>(` | none: a call line never starts with `class`/`interface` | S |
| 2 | Barrels aren't followed. `export { default as User } from "./User"` in `index.ts` sends the name to the search by name. Together with #1 this is what makes the WRONG jumps | 3 (contributes to 2 WRONG + 1 pick-miss) | through #1 | `plugins/webhooks/server/tasks/DeliverWebhookTask.ts:26` `GroupUser,` in `import {…} from "@server/models"` → `app/models/GroupUser.ts:12`; should be `server/models/GroupUser.ts:41` | When `imported_at` finds nothing, follow `search::reexports` / `behind_barrels` (already used for implementations, 4 levels deep), extended to `{ default as X }` and `{ x as X }` | low: each hop is written out in the source | M |
| 3 | A module that re-exports its default import (`import Text from "…"; export default Text;`) is landed on at the `export default` line | 1 | **yes** | `app/components/Notifications/NotificationListItem.tsx:79` `<Text weight="bold">` → `app/components/Text.ts:3` `export default Text;`; should be `shared/components/Text.tsx:27` | When the `^export default` fallback (D:704) finds `export default Name;` and the module's own imports bind `Name`, follow that import (up to 4 hops). An expression such as `observer(X)` stays on the line | low | S |
| 4 | A JSX attribute or an object-literal key is taken for the local of the same name: `bindings` finds it in scope and D:70 jumps | 1 (+2 WRONG repros) | **yes** | `app/components/Table.tsx:435` `<Placeholder columns={allColumns.length} …/>` → `Table.tsx:129` (the `columns` parameter of TableViewInner); should be `Table.tsx:626` `columns: number;` (Placeholder's props) | A word followed by `=` (not `==`/`=>`) inside `<Tag`, or by `:` at a literal's key position, is a property: skip locals and the bare-name search. It must refuse (still treat as a value): shorthand `{ columns }`, the two sides of a ternary, labels, type annotations. For JSX, resolve `Tag`, read its first parameter's type (`({ columns }: { columns: number })` or `: Props`) and take the field; otherwise answer "no definition" | low if the key test is strict | S (refuse) / M (props) |
| 5 | The search by name for a bare word includes another file's function-local `const/let/var` (`project_definitions` has no indent rule) | 1 pick-miss (+1 WRONG repro) | **yes** when there is exactly one | `server/queues/tasks/DuplicateCollectionDocumentsTask.test.ts:36` `actorId: user.id,` → picker of `const actorId = …` inside functions of 3 other files. Repro with a single such local: "actorId → later.actorId (by name, 1 match)" | A `const/let/var/function` hit in another file counts only at the top level (indent 0 or `export`), except inside `namespace` and `declare global` bodies | low | S |
| 6 | A global's member is looked up among the project's members. `JSON`, `Object` and `document` are neither bound nor declared, so the qualifier is treated as an unknown value | 1 | **yes**, only because there's no `node_modules` | `server/tools/documents.test.ts:489` `JSON.parse(…)` → `plugins/storage/server/utils/ByteRangeHelper.ts:20` `static parse(` "by name, 1 match"; should be `parse(text: string, …)` in TypeScript's `lib.es5.d.ts` | A qualifier no scope, import or project declaration binds, when it is a known global (`JSON Math Object Array Promise Reflect Number document window`) or capitalised: take members from the lib `.d.ts` only, otherwise "no definition" | low | S |
| 7 | `Enum.Member` and `Class.staticField` behind an import aren't found. `imported_at` greps only the declaration patterns (D:686), and neither `Admin = "admin",` nor `static x = …` matches them. An enum member with no value (`ZOOMED,`) has no rule anywhere | 4 (3 none, 1 pick-miss) | no | `server/policies/collection.test.ts:596` `CollectionPermission.Admin` → none; should be `shared/types.ts:239` `Admin = "admin",`. `app/models/Document.ts:708` `NavigationNodeType.Document` → a picker of `class Document`…; should be `shared/types.ts:530` | When the chain names a type, add `field_patterns` to that grep; `search::qualified == "Enum.Member"` keeps the hit inside the type. Enum body: `^\s*NAME\s*(=.*)?,?$` directly inside `enum X {` | none | S |
| 8 | A member of a default-imported instance: `imported_at` returns nothing for `default` plus a chain (D:669) | 2 none | no | `server/emails/templates/InviteReminderEmail.tsx:74` `env.APP_NAME` → none; should be `server/env.ts:877` `public APP_NAME = "Outline";` (the module ends with `export default new Environment();`) | Read the module's `export default` line: `new C(…)`, or a name whose binding reads `New(C)`, gives the type `C`; then do the typed member lookup, reported as `via env: Environment` | low | M |
| 9 | The key of a `const X = { … }` literal isn't a declaration ("the key of … an object literal is no field") | 1 none | no | `server/routes/api/documents/documents.ts:1393` `RateLimiterStrategy.TwentyFivePerMinute` → none; should be `server/utils/RateLimiter.ts:196` `TwentyFivePerMinute: {` | When the qualifier's binding (local, top-level or imported) is `const X = {`, scan that literal's depth-1 lines for `key:`, `key(` or `key,` | low: only inside the proven literal | S–M |
| 10 | A field of an inline type literal in a parameter's annotation isn't found | 1 pick-miss | no | `shared/editor/components/DisabledEmbed.tsx:15` `props.href` (with `props: Omit<Props,…> & { href: string }`) → 8 other `href` fields; should be line 7 | Treat each `{ … }` operand of a top-level `&` in the annotation as a type body | low | M |
| 11 | The file's own declaration doesn't win for a bare name. `names_itself` (D:62) drops `type/class/function` bindings from the locals, and the scope walk never sees a top-level declaration below the cursor | 4 pick-hit | no | `shared/components/EmojiText.tsx:24` `…}: Props) {` → "Props: by name, 462 declarations". `app/components/Notice.tsx:20` `<Container>` → 21 (the file's own `const Container` at line 42 is listed first) | Apply `names_itself` only to dotted chains. Add the file's top-level declarations, above or below the cursor, as the module scope. Overloads and declaration merging stay a picker of this file's lines | low | S |
| 12 | Gaps in receiver typing: a destructured typed parameter `({ apiKey }: Props)`; destructuring a call, `const { auth } = useStores()`; `for…of` over a chain such as `document.memberships`; a function that returns a local it constructed | 5 pick-hit | no | `app/scenes/Settings/components/ApiKeyListItem.tsx:98` `apiKey.id` → "by name, 66 declarations"; should be `app/models/base/Model.ts:69` via `apiKey: ApiKey` | Extend `Value::Field` to a parameter's pattern and to a call head, extend `Value::Element` to a chain, and allow one more return hop | moderate: these links currently break the chain on purpose | M each |
| 13 | An arrow function's return type is read as its single parameter: B:706 `(?:^\|[^\w$.])NAME\s*=>` matches `): NavigationNode =>`, and the fake local hides the import (D:66) | 1 pick-hit | no | `server/tools/util.test.ts:19` `): NavigationNode => ({` → "by name, 4 declarations"; should be the `via import` to `shared/types.ts:535` | The bare-parameter arrow form needs `(`, `,`, `=` or the start of a line in front of the name. It must refuse `:` | none | S |
| 14 | False declarations fill the pickers. A call that passes a callback matches the method pattern (defs.rs:378): `action((e) => {`, `it("…", async () => {`. A wrapped call `map(` at the end of a line matches too. `  type NodeSpec,` in a wrapped import list matches the `type` alias pattern (defs.rs:85) | contributes to 3 | no; it turns a call into "at a declaration" | `app/components/WebsocketProvider.tsx:348` `action((event…) => {` → "action: at a declaration, 35 others by name". `server/routes/api/searches/searches.test.ts:138` `it("…", async () => {` → "4238 others" | The first thing inside `(` must be a parameter, never a string, `(`, `async (` or a literal. `name(` at the end of a line is a method header only when its closer at the same indent is `) {` or `): T {`, never `)`, `);` or `),`. `type X,` is no alias; an alias needs `=` or `<` | low | S–M |
| 15 | Beyond rules: contextual typing | 3 | no | `server/utils/ZipHelper.ts:314` `entry.fileName` in `this.walk(p, async (entry) => …)`; `server/tools/fetch.test.ts:204` `res!.result!.content![1].text` | none: "by name" is the right answer | — | — |
| env | An import of a package that isn't installed is treated as a workspace package (`own_module`, D:196), so the project is searched by name | 5 pick-miss (counted as artifacts) | no | `RevisionViewer.tsx:157` `observer(…)` from `mobx-react` → other files' `const observer = new ResizeObserver(…)` | For a bare specifier that no workspace declares and no `node_modules` has, land on the import line: "via import mobx-react (not installed)" | low | S–M |

Smaller issues:
- `...rest` binds nothing, because `names()` (`syntax.rs:292`) counts `.` as part of a name (ts0228, 1 none, S).
- On a wrapped destructuring or parameter list, merl lands on the statement's first line rather than the line that holds the name (ts0158, ts0163). A second `d` from there can't continue.

## 3. Pick-hit: why a picker, what narrows it

| rows | why a picker | what narrows it |
|---|---|---|
| ts0170, ts0198, ts0252, ts0258 | the file's own `type Props` or `const Container` loses to hundreds of namesakes; the right one is listed first | #11: jump (S) |
| ts0012 | the arrow's return type hides the import | #13 (S) |
| ts0018, ts0118 | destructured typed parameter `({ apiKey }: Props)` | #12 (M) |
| ts0221 | `const { auth, ui } = useStores()`; the chain broke at `auth` | #12 (M) |
| ts0291 | `for (const membership of document.memberships)`: the collection is a chain | #12 (M) |
| ts0061 | `getTestServer()` returns `server = new TestServer(app)` | #12 (M) |
| ts0014, ts0062 | a callback parameter typed by its context | beyond rules |
| ts0246 | the 3 overloads of one function | correct as it is |

## 4. Latency

No row takes over 300 ms (p50 61 ms, p90 153 ms, max 267 ms), because without `node_modules` nothing outside the project is searched. Of the 33 rows over 150 ms:
- 32 are member lookups by name for common fields (`id` 66 declarations, `collectionId` 67). Each is a members grep plus a fields grep over the project, then every field hit's file is read again to find its enclosing type.
- 1 is `it(` at 160 ms (cause #14).

With dependencies installed, expect the JavaScript numbers below.

## 5. Top improvements

**1. `d`: a class or interface whose type parameters prettier wrapped is dropped as a wrapped call**
`show_definitions` removes a hit ending in `<` unless it finds `>(…) {`. So outline's models and `interface APIContext<` vanish: "no definition", or a jump to the client-side namesake. That's 3 WRONG, 4 none and 1 pick-miss in this sample. Keep a `<`-ending hit that `declares_type` matches, or that has `extends`/`implements` before the `<`.
```ts
// server/models/User.ts: class User extends ParanoidModel<⏎  InferAttributes<User>⏎> {}  export default User;
// server/models/index.ts: export { default as User } from "./User";
// app/models/User.ts:     class User extends Model {}
import { User } from "./models";
User.build();   // d on User → app/models/User.ts:1 "by name, 1 match"; should be server/models/User.ts:1
```

**2. `d`: follow a barrel's re-exports and a module that re-exports its default import**
`import { User } from "@server/models"` goes by name, and `import Text from "~/components/Text"` stops at `export default Text;` in the re-exporting file (a WRONG jump). Reuse `behind_barrels` and `search::reexports` in `imported_at`, adding `{ default as X }`. When the default fallback lands on `export default Name;` and that module imports `Name`, follow the import.
```ts
// shared/Text.tsx: const Text = styled.span``; export default Text;
// app/components/Text.ts: import Text from "../../shared/Text"; export default Text;
import Text from "./Text";
<Text />   // d → app/components/Text.ts:3; should be shared/Text.tsx:1
```

**3. `d`: a bare name the file declares wins over namesakes elsewhere**
`names_itself` (D:62) drops the file's own `type`, `class` and `function` bindings, and declarations below the cursor are never seen. The result is "Props: by name, 462 declarations". For a bare word, keep the enclosing scopes' declarations and add the file's top-level ones, above or below. The same fix covers 22 of 60 JavaScript pick-hits.
```tsx
type Props = { title: string };          // other.tsx declares its own Props and Container
function Card({ title }: Props) {         // d on Props → "by name, 2 declarations"
  return <Container>{title}</Container>;  // d on Container → "by name, 2 declarations"
}
const Container = styled.div``;
```

**4. `d`: a JSX attribute or an object-literal key isn't the local of the same name**
Both jump to a `columns` parameter in scope ("columns: local"). With no local, a key jumps "by name, 1 match" to another file's function local. Treat `word=` inside a tag, or `word:` at a key position, as a property. Answer from the tag's props type when it resolves, otherwise "no definition". Shorthand `{ columns }` stays a reference.
```tsx
function Row({ columns }: Props) {
  schedule({ columns: 1 });            // d on the key → "columns: local" (line 1)
  return <Cell columns={columns} />;   // d on the attribute → "columns: local" (line 1)
}
```

**5. `d`: `Enum.Member`, `Class.static` and a default-imported instance's members, behind an import**
`imported_at` greps only the declaration patterns, so these come back "no definition" (5 rows here). When the chain names a type, add the field patterns and an enum-body rule (`NAME,` or `NAME = …` inside `enum X {`). Read a default export's `new C()` as the type `C`.
```ts
// shared/types.ts: export enum CollectionPermission { Read = "read", Admin = "admin" }
// src/env.ts: class Environment { public APP_NAME = "Outline"; } export default new Environment();
import { CollectionPermission } from "../shared/types";
import env from "../src/env";
CollectionPermission.Admin;   // d on Admin → "no definition"
env.APP_NAME;                 // d on APP_NAME → "no definition"
```

---

# REPORT-js: `d` in JavaScript (eslint, compared against tsserver)

eslint is CommonJS, with `node_modules` installed (803 packages, 368 MB). I read all 11 WRONG, 5 none and 29 pick-miss rows, 15 of the 60 pick-hits chosen to cover different words and files, and also checked all 60 pick-hits and all 58 rows over 300 ms. J1–J6 and J8 were reproduced in scratch projects.

## 1. Corrected tally

| verdict | bench | real | artifact | what the artifacts are | artifact share |
|---|---|---|---|---|---|
| WRONG | 11 | 9 | 2 | js0055: merl lands on `function handleImportsExports(` (line 262), and the parameter `modules,` is 2 lines below. js0091: `astUtils` → its `require` line, the way merl lands on an import line for a Go package; the oracle picks `module.exports = {` | 18% |
| none | 5 | 4 | 1 | js0082 `process.env.X`: the oracle's target is an index signature in `@types/node`, from tsserver's own type cache | 20% |
| pick-miss | 29 | 22 | 7 | 5 with the cursor on a method declaration (`fix(fixer) {` ×4, `onUnreachableCodePathSegmentEnd(segment) {`); `module.exports`, where the oracle answers the file's line 6; a key whose "definition" is a JSDoc `@returns {{…}}` | 24% |
| pick-hit | 15 | 14 | 1 | js0096, the cursor on `create(context) {` | 7% |

After removing artifacts: **ok +2, WRONG 9, none 4, pick-miss 22 (9 of them beyond rules), pick-hit 14**. 9 rows can't be scored.

## 2. Causes, most harmful first

| # | cause | count | WRONG? | example (merl → should) | fix sketch | risk | size |
|---|---|---|---|---|---|---|---|
| J1 | A CommonJS `require` binding is treated as a local. `search::imports` already binds `const x = require()` and `const { a } = require()`, but `search::bindings` returns the same statement as a local. That local hides the import (D:66) and wins at D:70 | 8 (7 WRONG, 1 none) | **yes** | `lib/eslint/eslint.js:1238` `getNamespaceFromTerm(…)` → `eslint.js:51` `const {` ("local"); should be `lib/shared/naming.js:95` `function getNamespaceFromTerm(term) {`. `lib/linter/code-path-analysis/code-path-state.js:763` `CodePathSegment.flattenUnusedSegments(` → the require at line 12; should be `code-path-segment.js:45` `class CodePathSegment {`. `lib/rules/no-unneeded-ternary.js:204` `astUtils.isCoalesceExpression(` → none | In the locals filter (D:62), treat a statement whose value is exactly `require("…")` or `require("…").x` as an import line, reading a wrapped `const {` to its `;`. It must refuse a require whose result is called (`require("debug")("eslint:cli")`), which stays local. For `const X = require("./x")` with `module.exports = Name` in x.js, resolve `X` and `X.m` through `Name` (today the `*` binding with an empty chain returns nothing). Bind `{ a: b }`: the parser only knows `a as b`. Note: 3 of the 7 WRONG rows (`echo` from `shelljs`) point into tsserver's own type cache, which merl can't reach even after this fix | low: the oracle follows these aliases the same way | S |
| J2 | An object-literal key is taken for the local of the same name; the key of a local or module-level literal isn't a declaration | 3 (1 WRONG, 1 none, 1 pick-miss) | **yes** | `lib/rules/comma-dangle.js:337` `node: lastItem,` in `context.report({` → `comma-dangle.js:318` `function forceTrailingComma(node) {`; the honest answer is "no definition". `lib/rules/array-callback-return.js:445` `messageAndSuggestions.messageId =` → 5+ `.d.ts` fields; should be line 435 `const messageAndSuggestions = { messageId: "", … }`. `lib/rules/quotes.js:54` `QUOTE_SETTINGS.backtick` → none; should be line 30 | Same as TS #4 (a key position means no locals and no bare-name search) and TS #9 (a proven `const x = {` literal's depth-1 keys) | low | S / S–M |
| J3 | JSDoc types aren't read: `/** @type {T} */`, `@param {T} x`, `@returns {T}`, `@typedef {Object} T` with `@property` | 4 (1 WRONG, 2 pick-miss, 1 pick-hit) | **yes**: a unique `.d.ts` field of another type gets the jump | `lib/cli.js:493` `options.maxWarnings` (with `/** @type {ParsedCLIOptions} */ let options;`) → `lib/types/index.d.ts:1354` of `ESLint.MaxWarningsExceeded`, "by name, 1 match"; should be `lib/options.js:43` `@property {number} maxWarnings` | (a) S: in `.js` files, a `@property {T} name` line in a `@typedef` block is a field line for the search by name. js0259 becomes a pick-hit. (b) M: in `.js` files read `@type`, `@param` and `@returns {T}` as annotations, `@typedef {import("./x").T} T` as an alias, and `@typedef {Object}` plus `@property` as a type body. It must refuse anything but a plain name `{T}` | medium for (b) | S + M |
| J4 | The search by name, and the member fallback "x was a namespace" (D:339), keep other files' function locals, inside the project and in `node_modules` | 1 pick-miss (+2 WRONG repros); contributes to 8 rows | **yes** when there is one | `eslint.config.js:66` `Object.values(…)` → 149 rows including `const values = this._values;` (`lib/rules/indent.js:156`). Repro with one such local: `Object.values(o)` → "values → f.values (by name, 1 match)" in another file | Same as TS #5, and apply it outside the project too (the function locals of acorn, @babel/parser and highlight.js show up today) | low | S |
| J5 | A quoted visitor method isn't read as a method header, so its parameter is unbound (`TS_METHOD`, B:637, takes only a `[\w$]+` name). With J4, that is enough for a WRONG jump | 2 pick-miss (+1 in the pick-hits; 1 WRONG repro) | **yes**, together with J4 | `lib/rules/indent.js:1218` `[...node.elements]` under `"ArrayExpression, ArrayPattern"(node) {` → "node: by name, 78 declarations", first `let node = leafNode` of another function; should be line 1215. Repro: `node.callee` under `"NewExpression:exit"(node) {` → another rule's `const callee = node.callee;` | Accept a quoted or `[computed]` name before `(` | none | S |
| J6 | Declarators continued over lines (`const a = …,⏎\tb = require(…),⏎\tc = …;`) aren't read. Only the first declarator binds, and only a `require` right after `const` is an import | 6 (1 none, 3 pick-miss, 2 pick-hit) | no | `lib/rules/semi-spacing.js:98` `sourceCode.isSpaceBetween` → "sourceCode: by name, 344 declarations"; should be line 78. `lib/cli.js:412` `log.error(` → 6 declarations; should be the require at `cli.js:24`, then `lib/shared/logging.js:11`. `newline-after-var.js:245` `nextLineNum` → none after 581 ms | Read a `const/let/var` statement to its `;`, split it at top-level commas, and bind each `name =`. Treat `, name = require("…")` as an import. It must refuse a continuation line whose statement doesn't start with a declaration keyword | low | S |
| J7 | The file's own nested `function report()` or top-level `class Config` loses to namesakes (`names_itself`, D:62). A `const report = () => …` in the same place does jump | 4 pick-hit here, **22 of 60 pick-hits in the whole run** | no | `lib/rules/no-extra-parens.js:1534` `report(node.argument)` → "report: by name, 66 declarations" (line 541 listed first) | Same as TS #11 | low | S |
| J8 | A wrapped class header hides the fields of its body. `enclosing_type` (`fields.rs:199`) stops at a `}> {` line, so the members `class SourceCode implements TextSourceCode<{…}> {` declares as fields are never collected | 4 pick-miss | no | `lib/rules/semi.js:377` `sourceCode.getTokenAfter(` → 5 declarations (the JS implementation in `token-store/index.js:397` plus 4 outside); missing is `lib/types/index.d.ts:306` `getTokenAfter: SourceCode.UnaryCursorWithSkipOptions;` | Skip a closer line that starts `}` + `>`, or read the header with `ts_header` | none | S |
| J9 | False declarations: a wrapped call `name(` at the end of a line; `type Config,` in an import list | contributes to 3 | no | `lib/rules/max-lines-per-function.js:199` `isFullLineComment(` (a call) → "at a declaration, 2 others by name" | Same as TS #14 | low | S–M |
| J10 | Builtins are read from every copy of `typescript` (`node_modules/typescript` and `@arethetypeswrong/core/node_modules/typescript`) | 2 pick-hit | no | `lib/rule-tester/rule-tester.js:1017` `Function.bind` → "Function: by name, 72 declarations", 717 ms | For a word no import binds, read one copy of each package, the nearest one | low | S–M |
| — | Beyond rules: contextual typing. A rule's `context`, `sourceCode = context.sourceCode`, a visitor's `node` and callback parameters get their types from the module's JSDoc `@type {Rule.RuleModule}`. The targets are fields in `@types/estree` or `@eslint/core`, outside the project, which merl doesn't collect by design | 15 (9 pick-miss, 1 none, 5 pick-hit) | no | `lib/rules/no-native-reassign.js:63` `context.sourceCode`; `lib/rules/indent-legacy.js:1304` `node.params` | none: "by name" or "no definition" is the right answer | — | — |

Smaller issue: an optional method signature wrapped over lines (`onUnreachableCodePathSegmentEnd?(` ⏎ … ⏎ `): void;`) isn't a declaration. The method pattern has no `?` before `(`, and the signature pattern needs everything on one line.

## 3. Pick-hit: why a picker, what narrows it

| rows | why a picker | what narrows it |
|---|---|---|
| js0019, js0162, js0069, js0255 (and 18 more across the run) | the file's own `function report` or `class Config` isn't preferred | J7 (S) |
| js0102, js0266 | `Traverser` / `ForkContext` are bound by a continued `, X = require(…)` | J6 + J1 (S) |
| js0013, js0033 | builtins found in two copies of typescript (72 rows, 0.6–0.7 s) | J10 (S–M) |
| js0185 | `const state = CodePath.getState(codePath)`; the type sits only in a JSDoc `@returns {CodePathState}` | J1 + J3b (M) |
| js0001, js0028, js0075, js0145, js0231 | receiver typed by context | beyond rules |

## 4. Latency

58 of 270 rows take over 300 ms (p50 50 ms, p90 621 ms, max 2.9 s), and 56 of those have `node_modules` candidates.
- **41 are member lookups by name** that reach `node_modules`: `length` 1.07–1.22 s (167 declarations), `parent` 0.76–1.0 s (174), `value` 0.52–1.27 s, `name` 1.22 s (297). merl greps 6,179 declaration files (28 MB, including two copies of typescript's lib). Then `show_definitions` reads each candidate's file again to drop lines inside comments (up to 500 candidates).
- **17 are bare words the project doesn't declare**, searched through all of `node_modules`. One `rg` pass over it takes about 0.7 s on this machine by itself.
  - `Statement` (2.9 s) and `In` (0.76 s) are words inside a regex literal. That's a sampler artifact, but merl could refuse a word inside a string or regex before searching.
  - `nextLineNum` and `NUMBERS` (0.58 and 0.69 s) are continued declarators the file does declare; with J6 they resolve in milliseconds.
  - `Function`, `SharedArrayBuffer`, `RangeError` and friends are read twice, once per typescript copy (J10).
- Possible fixes: J6; one copy per package; cache the comment-line scan for files outside the project, which don't change during a session.

## 5. Top improvements

**1. `d`: a CommonJS `require` binding is an import, not a local**
The import parser already reads `require`, but the scope walk returns the same statement as a local. The local hides the import and gets the jump: 7 WRONG and 1 none in this sample. Treat a statement whose value is exactly `require("…")` as an import line, and keep `require("x")(…)` local. Resolve `module.exports = Name` for a whole-module binding.
```js
// lib/utils/m.js:   function helper(n) { return n; }  module.exports = { helper };
// lib/utils/seg.js: class Segment { static make() {} }  module.exports = Segment;
const { helper } = require("./utils/m");
const utils = require("./utils/m");
const Segment = require("./utils/seg");
helper(1);        // d → "helper: local" (this line 1); should be m.js:1
utils.helper(1);  // d → "no definition"
Segment.make();   // d on make → "by name, 2 declarations"; should be seg.js
```

**2. `d`: read declarators continued over lines**
Only the first declarator of a `const` statement binds, so eslint's `const fs = require(…),⏎\tlog = require(…)` leaves `log`, `sourceCode` and `nextLineNum` unbound: "by name, 344 declarations", or "no definition" after 0.6 s spent in `node_modules`. Read the statement to its `;` and bind each `name =`, with `name = require(…)` as an import.
```js
const fs = require("node:fs"),
	helpers = require("./helpers"),
	LIMIT = 10;
helpers.go(LIMIT);   // d on helpers → "no definition"; d on LIMIT → "no definition"
```

**3. `d`: a nested `function` declared in the enclosing scope wins**
22 of the run's 60 pick-hits ask for a function that the enclosing `create(context)` declares. The scope walk finds it, but `names_itself` (D:62) drops it, while a `const report = node => …` in the same place jumps. For a bare word, keep the function and class declarations of the enclosing scopes as locals.
```js
module.exports = { create(context) {
	function report(node) {}          // another rule declares its own report
	return { Program(node) { report(node); } };   // d → "report: by name, 2 declarations"
} };
```

**4. `d`: in the search by name, another file's function locals are never candidates**
For a bare word, and for an unknown receiver with no member of that name (`Object.values`), merl keeps `const|let|var name` at any indent, and one match jumps. Keep another file's hits only at the top level (indent 0 or `export`, namespaces aside), outside the project as well. Also read a quoted visitor `"Program:exit"(node) {` as a method header (B:637): its `node` is what currently falls through to this path.
```js
// lib/a.js
function f(o) { const values = o.x; return values; }
// lib/b.js
Object.values(o);   // d on values → "values → f.values (by name, 1 match)" → a.js (WRONG)
```

**5. `d`: an object-literal key isn't the local of its name**
`context.report({ node: lastItem })` jumps to the enclosing `node` parameter ("node: local"). Treat a word followed by `:` at a literal's key position as a property: skip locals and the bare-name search. For `x.key` where `x` is a proven `const x = { key: … }`, read the literal's depth-1 keys. Shorthand `{ node }` stays a reference.
```js
function forceTrailingComma(node) {
	context.report({ node: lastItem });   // d on the key → "node: local" (the parameter)
}
const messageAndSuggestions = { messageId: "" };
messageAndSuggestions.messageId = "x";    // d on messageId → .d.ts fields by name; should be the key above
```
