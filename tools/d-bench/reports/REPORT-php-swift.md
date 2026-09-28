# REPORT-php.md

# `d` in PHP: koel (Laravel, no `vendor/`), oracle intelephense

Scored: ok 36, pick-hit 46, WRONG 0, pick-miss 25, none 31. Not scored: 161 rows where the oracle had no answer (mostly framework calls, since there is no `vendor/`).

## Tally of the rows read

| verdict | read | artifact | unreachable | real |
|---|---|---|---|---|
| WRONG | 0 of 0 | – | – | 0 |
| hidden WRONG (merl jumped, oracle had no answer) | 11 of 11 | 0 | – | **11** |
| none | 31 of 31 | 8 (namespace segment) | 19 (PHP built-ins) | 4 |
| pick-miss | 25 of 25 | 8 (7 namespace segments, 1 facade `@method`) | 0 | 17 |
| pick-hit | 17 of 46 | 0 | 0 | 17 (every one can be narrowed) |
| ok | 0 of 36 | – | – | all 36 are `by name, 1 match`; PHP has no proven jump today |

- **Artifact or unreachable share:** none 87% (27/31), pick-miss 32% (8/25), pick-hit 0%.
- **Artifacts:**
  - The cursor is on a namespace segment of a `use` line, such as `Http` in `use App\Http\Requests\…\UploadSongRequest;`. The oracle answers the class the whole name leads to.
  - `License::isPlus()`: the oracle answers the facade's `@method` docblock line. merl's picker holds the real `LicenseService::isPlus`.
- **Unreachable:** built-ins such as `trim`, `sprintf` and `Exception` exist only in intelephense's stubs. "no definition" is the right answer.
- **Hidden WRONG:** these 11 rows cannot be scored because `vendor/` is missing. A test project with a fake `vendor/` (`work-php/r2`) shows merl still picks the project's namesake, so they are wrong in a real install too.

## Causes, most harmful first

| # | cause | count (sample) | WRONG? | example | fix sketch | risk | size |
|---|---|---|---|---|---|---|---|
| 1 | `->` is not read as a member access. `$x->word` is searched like a bare word, so every `$word = …` of any function in any file is a candidate, and nothing from `vendor/` joins while the project has a namesake | hidden WRONG 8. In the run, 1,285 of 1,766 candidates (73%) in member pickers are local assignments | **yes** | `FavoriteService.php:102` `->where('user_id', $user->id)` jumped to `Models/Artist.php:116` `$where = ['name' => $name];` | read `->` and `?->` as `.`, including in `qualifier()`. Add PHP member patterns and field rules. `->word(` looks at methods only. Members from `vendor/` join the picker | low | S–M |
| 2 | A PHP `use` is never followed: `imported_at` returns an empty module for PHP. The Rust-shaped `brings`/`outside` test also reads every `use App\…` as an outside import, which turns off `Class::word` narrowing | pick-hit 4. With `vendor/` present, `Arr::get` jumps to the project's own `get` (r2) | **yes** (with `vendor/`) | `SongRepository.php:149` `Song::query(type: …)` gives `by name, 16 declarations`. The same cursor without the `use` line gives `query → Song::query (via Song)` | resolve PSR-4 from composer.json `autoload(-dev).psr-4` (`App\\` → `app/`). A prefix outside the map searches outside first. A bare class name is the file's `namespace` plus the name | low | M |
| 3 | `$this->`, `self::`, `static::` and `parent::` do not read the class around the cursor | pick-hit 4. In the run, 20 of 46 pick-hits are `$this->word` answered in the same file. hidden WRONG 1 | **yes** (1) | `SongResource.php:113` `$this->song->album_name` gives 212 candidates, 197 of them `$song = …`. The answer is line 81, `protected Song $song,` | look in the class, then its `use Trait;` lines, then `extends` (resolved as in #2). A base class in `vendor/` gets a lookup narrowed to it, never another class's namesake | low | M |
| 4 | Typed class constants (PHP 8.3) have no rule: the `const` pattern expects the name right after `const`. All 85 class constants in koel are typed | none 1 | no | `SongRepository.php:166` `self::LIST_SIZE_LIMIT` gives `no definition`. The answer is line 39, `private const int LIST_SIZE_LIMIT = 500;` | `{mods}const\s+(?:\??[\w\\|]+\s+)?{w}\b` | none | S |
| 5 | Docblock tags `@property`, `@property-read` and `@method` have no rule. Eloquent attributes, FormRequest input and facades are declared this way | none 3, pick-miss 12 | no | `SongService.php:117` `$song->title` gives 3 candidates: a local `$title = trim(…)` and two value-object properties. The answer is `Models/Song.php:70` ` * @property string $title` | a field or method of the class under the docblock: `^\s*\*\s*@property(?:-read\|-write)?\s+.*?\$word\b` and `^\s*\*\s*@method\s+(?:static\s+)?.*\bword\s*\(` | low | S |
| 6 | A named argument `name: $x` is read as a bare word, so every `$name =` is a candidate | pick-miss 5, pick-hit 2 | no | `PlaylistSongController.php:48` `getMany(ids: $request->songs, …)` gives 15 `$ids =` lines. The answer is `SongRepository.php:317` `getMany(array $ids, …)` | `word:` (not `::`) right after the `(` or `,` of an open call is a parameter. Resolve the callee and land on `$word` in its signature | low | M |
| 7 | Namespace segments: the `namespace …\Word;` pattern matches the last segment alone, one row per file | 15 artifacts, hidden WRONG 2 | **yes** (2) | `config/app.php:32` `use Illuminate\Support\Facades\Route;` on `Support` jumped to `NullQrCodeProvider.php:3` `namespace App\Services\Auth\Support;`. `Models` opens a picker of 32 `namespace App\Models;` lines | a segment matches a namespace only with its full written prefix, and the rows collapse to one. A word not followed by `\` is never a namespace | low | S |
| 8 | Same-line misfire: `at a declaration` fires because the line declares the word, although the cursor is on another occurrence of it | pick-miss 1 | no | `PlaylistCollaboratorController.php:34` `$collaborator = …($request->collaborator)`, on the second word, gives `at a declaration, 16 others` | say `at a declaration` only when the cursor's word range is the declared name | low | S |
| 9 | Receivers are never typed, although PHP writes the types: parameter hints, typed and promoted properties, `new X`, `): X` | pick-hit 3 | no | `DeletePodcastIfNoSubscribers.php:17` `$event->podcast` with `handle(UserUnsubscribedFromPodcast $event)` gives 39. The answer is `UserUnsubscribedFromPodcast.php:12` | add PHP to `typed.rs`: typed parameters and properties, `new X`, `X::make()` through `): X`. Refuse unions, `static`/`self` returns and generic docblocks | medium | L |
| 10 | PHP built-ins have no source on disk | none 19 | – | `trim(…)`, `new Exception(…)` | beyond rules: "no definition" is right | – | – |

### Details, and what each fix must refuse

1. **`->` as a member access**
   - Wrong jumps to locals:
     - `->filter(` at `KoelAssistant.php:76` landed on `tests/…/HookRegistryTest.php:239 $filter = …`.
     - `->where(`, twice.
     - `app()->build(` landed on `AddBuildHeader.php:20 $build = …`.
   - Wrong jumps to a project namesake of a framework member:
     - `$join->on(`, twice, landed on `DirectoryScanner::on`.
     - `$exceptions->render(` landed on `SubsonicAwareErrorRenderer::render`.
     - `$request->input('prompt')` landed on the property `WatchRecord.php:24 protected string $input;`, although the cursor is on a call.
   - Member patterns, all indented: `function word(`; `$word` behind `public|protected|private|var|readonly|static`; a promoted constructor parameter; the tags of #5.
   - Refuse: `$word =`, a function in column 0, classes, namespaces and constants, and a property for `->word(`.
   - With member patterns set, the existing `external_grep` adds `vendor/` members. An untyped `$join->where(` then opens a picker instead of jumping to a namesake.
2. **PSR-4 resolution**
   - `use App\Models\Song` resolves to `app/Models/Song.php`. The word is looked up there directly, or as `Song.word` for `Song::word`, and reported as `via import`.
   - A prefix outside the map makes `imported_at` return `None`, so the lookup goes outside first. That path already works when the project has no namesake: r2 gives `get → Arr::get (via import Illuminate/Support/Arr)`.
   - Refuse `use function` and `use const`, and classmap-only directories (koel's `database/`): those stay by name.
   - The `brings`/`outside` check in `definition.rs` must not count a PHP `use` of a project namespace as outside.
3. **The class around the cursor**
   - It is the nearest `class|trait|enum|interface X` above the cursor; PSR-4 keeps one per file.
   - Refuse `new class` (an anonymous class). Inside a trait, `$this` is any class that uses the trait, so the lookup stays by name among those classes.
   - `self::INVALID` at `ScanCommand.php:184` jumped to `LicenseStatus::INVALID`. The class extends Symfony's `Command`, so the right answer is `vendor/` or "no definition".
5. **Docblock tags**
   - The owner is the class under the docblock: scan down to the `class` line to build the qualified name.
   - Refuse `@param` and `@var` in a function's docblock.
6. **Named arguments**
   - The callee is found this way:
     - `new X(` and `new self(`: `__construct`.
     - `X::m(` and `$this->m(`: through #2 and #3.
     - Anything else: by name, and only when it has exactly one declaration.
   - A promoted parameter lands on its property line; the oracle agrees for `image` and `name`.
   - Refuse a ternary `?:`, a `case X:` and a statement label.
   - When the callee is not resolved, offer only, or say "no definition". Never list the `$name =` lines.

## Pick-hit rows: why a picker, and what narrows it (17 read)

| reason | rows | narrowed by |
|---|---|---|
| `$this->word` declared in the file's class (`put`, `song`, `context`, `encyclopediaService`) | 4 (20 of 46 in the run) | #3 makes it a proven jump |
| `Class::m` with namesakes of the class or the member (`PaginationStrategyResolver::resolve`, `Song::query`, `AlbumResource::toArray` from the same namespace, `use App\Facades\Dispatcher` beside `App\Services\Dispatcher`) | 4 | #2 |
| a type name next to `namespace …\Name;` lines (`Genre`, `User`, `PlaylistFolder`, `Album`, each with a `use App\Models\X`) | 4 (4 in the run) | #2, or #7 alone: dropping the namespace rows leaves exactly the oracle's class |
| a typed receiver (`$event->podcast`, `$info->toArray()`, `$this->artistRepository->getRecentlyAdded`) | 3 | #9 (the last one needs #3 plus the promoted property's type) |
| a named argument on a promoted constructor parameter (`new self(name: $name, image: $image)`) | 2 | #6 |

## Latency

- p50 22.7 ms, p90 30.4 ms, max 138.7 ms. No row is over 300 ms.
- The max is the first cursor, `php0000`, on a cold cache.
- `Class::word` rows take 40–70 ms: they grep the project a second time for the owner, and compile the `brings` regex on every call.

## Top improvements

1. **PHP: `$x->name` is a member access, not a bare word**
   - `->` and `?->` count as `.`, and `$this` becomes the chain `["this"]`.
   - Member patterns: methods, properties, promoted parameters and `@property` tags. Never `$name =` locals, column-0 functions or namespaces.
   - `->name(` looks at methods only.
   - `vendor/` members join the picker, as in Python and TypeScript.
   - Repro:
     ```php
     class Other { public function run() { $where = ['a' => 1]; } }        // app/Services/Other.php
     class Repo  { public function run($join) { $join->where('a', 'b'); } } // d on `where`
     ```
     Today: `where → Other::run::where (by name, 1 match)`. Expected: a picker of `where` methods from the project and `vendor/`, or "no definition".
2. **PHP: follow `use` and the file's `namespace` through composer.json's PSR-4 map**
   - `use App\Models\Song` resolves to `app/Models/Song.php` (`via import`), and `Song::query` to `Song.query` in that file.
   - A prefix outside the map searches `vendor/` first, so the project's namesakes are no longer candidates.
   - The Rust `use … outside` test must stop disabling `Class::` narrowing for PHP.
   - Repro:
     ```php
     // app/Models/Song.php:  namespace App\Models; class Song  { public static function query() {} }
     // app/Models/Other.php: namespace App\Models; class Other { public static function query() {} }
     namespace App\Repos; use App\Models\Song; … Song::query();   // d on `query`
     ```
     Today: `by name, 2 declarations`. Expected: `query → Song::query (via import app/Models/Song.php)`.
   - With `vendor/`, `use Illuminate\Support\Arr; … Arr::get()` beside a project `function get(` jumps to the project's `get`.
3. **PHP: `$this->`, `self::`, `static::` and `parent::` read the class around the cursor, its traits and its parents**
   - The order is the file's class, then its `use Trait;` lines, then `extends` (resolved as in 2).
   - A parent in `vendor/` gets a lookup narrowed to that parent, never another class's namesake.
   - This fixes 20 of 46 pick-hits, and stops `self::INVALID` landing on an unrelated enum case.
   - Repro:
     ```php
     class ImageStorage { function store() { $this->put('a', 'b'); } private function put(string $f, string $c) {} }
     class LambdaSongController { public function put($r) {} }   // d on `put` in store()
     ```
     Today: `put: by name, 2 declarations`. Expected: `put → ImageStorage::put (via $this)`.
4. **PHP: typed class constants and docblock `@property`, `@property-read` and `@method` tags are declarations**
   - `const int NAME`: all 85 of koel's constants are typed, and today none of them is found.
   - ` * @property T $name` above a class is a field of that class. This is where Eloquent columns and FormRequest input are declared.
   - Repro:
     ```php
     class R { private const int LIMIT = 500; function f() { return self::LIMIT; } }  // d on LIMIT: "no definition"
     /** @property string $title */ class Song extends Model {}   // $song->title: nothing
     ```
5. **PHP: a named argument `name:` names a parameter of the callee**
   - For `f(ids: $x)`, resolve the callee (`new X(` goes to `__construct`; `X::m(` and `$this->m(`; otherwise a single by-name match) and land on `$ids` in its signature. A promoted parameter lands on its property line.
   - When the callee is not resolved, offer only. Never search the `$ids =` locals.
   - Repro:
     ```php
     class Repo { public function getMany(array $ids) {} }
     $ids = [];                   // elsewhere in the project
     $repo->getMany(ids: $x);     // d on `ids`: today every `$ids =`, expected `array $ids`
     ```

=====================================================================
# REPORT-swift.md

# `d` in Swift: Alamofire (SwiftPM library), oracle sourcekit-lsp

Scored: ok 44, pick-hit 94, WRONG 7, pick-miss 30, none 23. Not scored: 68 rows where the oracle had no answer, and 4 with the cursor on a declaration.

## Tally of the rows read

| verdict | read | artifact | unreachable (SDK or stdlib, no source) | real |
|---|---|---|---|---|
| WRONG | 7 of 7 | 0 | 0 | **7**: 5 jumps to a project extension of a Foundation type; 2 argument labels that land on the property 4–7 lines away from the init parameter |
| hidden WRONG (merl jumped, oracle had no answer) | 3 of 3 | – | – | 1: `Example/…/DetailViewController.swift:129` `Data(contentsOf:)` jumped to `Tests/TestHelpers.swift:484 extension Data {`. The other 2 are correct |
| none | 23 of 23 | 0 | 21 | 2 (parameters) |
| pick-miss | 30 of 30 | 0 | 18: 11 pickers of the project's extensions of `URL`, `URLRequest`, `DispatchQueue`, `Result` and `HTTPURLResponse`; 7 members of SDK types | 12 |
| pick-hit | 22 of 94 | 2 (overload sets, where the picker is the answer) | 0 | 20 |
| ok | 0 of 44 | – | – | 39 `by name, 1 match`, 5 `via Outer` |

Artifact or unreachable share: WRONG 0%, none 91%, pick-miss 60%, pick-hit 9%.

## Causes, most harmful first

| # | cause | count (sample) | WRONG? | example | fix sketch | risk | size |
|---|---|---|---|---|---|---|---|
| 1 | `extension X` counts as a declaration of `X`. Next to the type it crowds the picker; alone (a Foundation type) it gets the jump | WRONG 5 (+1 hidden), pick-miss 11. In the run, 27 of 94 pick-hits are a type plus its extensions, and dropping the extensions leaves exactly the oracle's line in all 27 | **yes** | `Source/Core/UploadRequest.swift:32` `case data(Data)` jumped to `Tests/TestHelpers.swift:484 extension Data {`. `Tests/ValidationTests.swift:386` `AFError?` opens a picker of 24 (the enum and 23 extensions) | a `class/struct/enum/protocol/actor/typealias X` line drops the `extension X` rows. With no such line, `X` is outside: never jump, say `X: declared outside the project, N extensions` over the picker | low | S |
| 2 | Function-body locals answer `x.word` and bare words: Swift has no member patterns and no scopes | pick-hit 3. In the run, 2,728 of 3,702 candidates (74%) in member pickers are a `let`/`var` inside a function, closure or block | no (floods) | `Tests/RequestTests.swift:164` `session.request(url)` gives 405 candidates. `…AuthenticationInterceptorTests.swift:1507` `.result` gives 279, 268 of them locals | a `let`/`var` whose nearest less-indented line above is not a type, extension or protocol header is a local. It is never a candidate behind a `.`, and for a bare word only inside its own function, above the cursor (#3) | low | S |
| 3 | Parameters, closure parameters and `if let`/`guard let`/`for`/`catch` bindings have no rule | none 2, pick-miss 8, pick-hit 2 | no | `Source/Core/SessionDelegate.swift:116` `…earlyWithError: error)` under `if let error = evaluation.error {` (line 115) gives 50 declarations of `error`. `RequestTaskMap.swift:48` `requestsToTasks[request]` gives 405; the answer is `subscript(_ request: Request)` on line 47 | Swift `bindings()`: walk the braces up from the cursor, as TS/Go `block_bindings` do. Binding forms are listed under the table | medium | M |
| 4 | Argument labels are read as bare words | WRONG 2, pick-miss 2 | **yes** (2: right type, wrong line) | `Tests/AuthenticationInterceptorTests.swift:979` `.init(interval: 30, maximumAttempts: 1)` jumped to `AuthenticationInterceptor.swift:180 public let maximumAttempts: Int`; the answer is the `init(interval:maximumAttempts:)` on line 187. `ResponseSerializationTests.swift:1110` `serializeDownload(request: nil, response: nil, …)` gives 267 | `word:` right after the `(` or `,` of an open call is a label. Resolve the callee (`Type(` or `Type.init(` to its `init`s, `f(` to `func f`) and land on the parameter whose external label is the word | low | M |
| 5 | Candidates the compiler cannot see: a test target from the library, another file's `private`/`fileprivate`, a type declared inside a function, a nested type outside its outer type | pick-hit 2, plus the extra rows in 2 WRONG and 3 pick-miss. In the run, 31 cursors in `Source/` were offered 780 `Tests/` candidates, and 18 cursors 66 private ones from other files | **yes** (through #1) | `Source/Features/MultipartFormData.swift:395` `EncodingCharacters.crlf` offers `Tests/MultipartFormDataTests.swift:29` too. `Result<URLRequest, any Error>` offers `OfflineRetrier.swift:248 enum Result`, which is nested in `struct PathMonitor` | from outside a test target (`Package.swift` `.testTarget(path:)`, `Tests/` by default), drop test files. Drop other files' `private`/`fileprivate`, and types whose enclosing line is a `func`. A nested `Outer.Name` answers a bare `Name` only inside `Outer` or its extensions | low (nested types: medium, since subclasses inherit them) | S |
| 6 | An implicit member `.word`, or `Type.word`, takes every declaration of the name | pick-hit 2 | no | `Tests/TestHelpers.swift:92` `case let .bytes(count):` gives 4 (`case bytes`, `static func bytes`, `let bytes: Int`, `var bytes`). `Endpoint.method(.post)` gives both `static func method` and `var method` | a leading-dot member and `Type.word` read only `case word` and `static`/`class` members. In a `switch` case pattern, only cases | low | S |
| 7 | Generic parameters have no rule | pick-miss 2 | no | `Source/Features/Combine.swift:262` `Handler<Value, AFError>` offers two `struct Value {}` declared inside test functions. The answer is `DataStreamPublisher<Value: Sendable>` on line 258 | the `<…>` of the enclosing headers (type, func, init, typealias) bind names like locals do | low | S |
| 8 | The enclosing type's own members do not come first (implicit `self`, superclass) | pick-hit 2 | no | `Source/Core/Protected.swift:39` `lock()` inside `extension Lock` gives 5, other types' `lock`s included; the answer is `Lock`'s `func lock()` on line 28. `timeout` in a `BaseTestCase` subclass gives 4; the answer is `Tests/BaseTestCase.swift:57` | a bare word in a type or extension body reads that type's members first (its body and its extensions), then the superclass named first in its header, when the project declares it | medium | M |
| 9 | Receivers are never typed, although Swift writes the types (`let encoder: URLEncodedFormEncoder`, `lhs: Instant`) | pick-hit 2. The 7 SDK-member pick-misses would say "outside" instead of offering namesakes | no | `Source/Core/Instant.swift:54` `lhs.value - rhs.value` in `static func -(lhs: Instant, rhs: Instant)` gives 34; the answer is `let value: Double` on line 35 | add Swift to `typed.rs`: annotations, `Type(…)`, `-> Type`, with `?`/`!` stripped. A type the project only extends ends the chain with "declared outside" | medium | L |
| 10 | Same-line misfire: the line declares the word, but the cursor is on another occurrence (shared with PHP) | pick-hit 2 | no | `Source/Core/Request.swift:520` `let validators = validators.read(\.self)` gives `at a declaration, 1 other`. `let request = session.request(…)` gives `at a declaration, 404 others` | say `at a declaration` only when the cursor is on the declared name | low | S |
| 11 | SDK, stdlib and XCTest have no source on disk | none 21, pick-miss 18 | – | `URLSession`, `Sendable`, `.utf8`, `wait(for:)` | beyond rules: "no definition" is right | – | – |

### Details, and what each fix must refuse

1. **Extensions.** The `extension X` rows stay reachable through `D`.
3. **Swift bindings**
   - Binding forms:
     - statement `let`/`var` of each enclosing block, above the cursor;
     - `if`, `guard` and `while` with `let` or `var x =`;
     - `for x in`;
     - `catch let x`, and a bare `catch {`, which binds the implicit `error`;
     - closure heads `{ a, b in`, `{ (a: T) in` and `{ [weak self] a in`;
     - the parameters of the enclosing `func`, `init` or `subscript`: `label name: T` binds `name`.
   - The shorthand `if let x {` rebinds the outer `x`, so the walk goes on outward.
   - A form the walk cannot read (a tuple pattern, `case let .x(name)`) must stop the walk, not hand the word to an outer name.
   - Never cross into a nested function.
4. **Argument labels**
   - Refuse:
     - a `[ … ]` dictionary literal;
     - a ternary `a ? b : c`;
     - `case .x:` and `default:`;
     - a closure's `{ (a: T) in`;
     - a declaration's own parameter list (a `func` or `init` in front of the `(`);
     - a tuple `(x: 1, y: 2)`, which has no callee.
   - When the callee is not resolved, say it is an argument label. Never search by name.
5. **Visibility**
   - `@testable import` lets tests see the library, not the other way round.
   - `Example/` is in no target of `Package.swift`; it sees the library and never `Tests/`.
6. **Implicit members.** A leading dot means a `.` after `(`, `,`, `case`, `=`, `return`, `:` or `[`.

## Pick-hit rows: why a picker, and what narrows it (22 read)

| reason | rows | narrowed by |
|---|---|---|
| a type plus its extensions (`AFError`, `Session`, `Request`, `ResponseCacher`, `Empty`, `ResponseSerializer`) | 6 (27 of 94 in the run) | #1: all 27 become a correct jump |
| flooded by other functions' locals (`session.request` 405, `.result` 279) | 3 | #2 cuts about 74%; a jump needs #9 |
| a local of the same function (`response`, `didReceive`) | 2 | #3 |
| the enclosing type or its superclass (`lock`, `timeout`) | 2 | #8 |
| visibility (the `Tests/` copy of `EncodingCharacters`; the nested `DataStreamTask.Stream` against `DataStreamRequest.Stream`) | 2 | #5 |
| an implicit or static member (`.bytes`, `Endpoint.method`) | 2 | #6 |
| a typed receiver (`lhs.value`, `encoder.encode`) | 2 | #9 |
| same-line misfire (`validators`; `request` too) | 1 | #10 |
| an overload set (`upload(stream, with:…)` 18, `serialize(_:forKey:)` 14): artifact | 2 | optional, size M: keep the overloads whose external labels match the call's, letting defaulted parameters be omitted |

## Latency

- p50 4.5 ms, p90 9.2 ms, max 34.3 ms. No row comes near 300 ms.
- The slowest rows are the 405-candidate `request` pickers: every candidate's file is read to rule out string and comment lines.

## Top improvements

1. **Swift: an `extension` is not the declaration of its type**
   - When the project declares `X`, drop the `extension X` rows. This turns 27 of 94 pick-hits into correct jumps.
   - Extensions alone mean `X` is outside: no jump, and the status line says so. This stops 5 of the 7 WRONG jumps.
   - Repro:
     ```swift
     // Source/Core/AFError.swift
     public enum AFError: Error { case explicitlyCancelled }
     extension AFError { var isCancelled: Bool { true } }
     let e: AFError? = nil                        // d on AFError
     // Source/Core/Upload.swift
     enum Uploadable { case data(Data) }          // d on Data
     // Tests/TestHelpers.swift
     extension Data { static let empty = Data() }
     ```
     Today: `AFError` opens a picker of the enum and the extension, and `Data` jumps to the `Tests/` extension. Expected: the enum, and `Data: declared outside the project` with the extensions in a picker.
2. **Swift: locals and parameters bind the name; another function's locals are never candidates**
   - `let` and `var` inside function bodies stop answering `x.word` and other functions' bare words; 74% of member candidates in the run are such locals.
   - A scope walk binds parameters, closure parameters, `if let`, `guard let`, `for` and `catch`.
   - Repro:
     ```swift
     func handle(_ evaluation: Evaluation) {
         if let error = evaluation.error {
             fail(error)            // d on error: today 50 declarations by name, expected line 2
         }
     }
     ```
3. **Swift: an argument label names a parameter of the callee**
   - Repro:
     ```swift
     struct RefreshWindow {
         let maximumAttempts: Int
         init(interval: Double = 30, maximumAttempts: Int = 5) { self.maximumAttempts = maximumAttempts }
     }
     let w = RefreshWindow(interval: 30, maximumAttempts: 1)   // d on maximumAttempts
     ```
     Today: a jump to `let maximumAttempts: Int` on line 2. Expected: the init's parameter on line 3.
   - For a callee with many declarations, such as `serializeDownload(request:response:…)`, it lands on that function's parameter instead of 267 `response` declarations.
4. **Swift: what the compiler cannot see is no candidate**
   - A test target from the library, another file's `private` or `fileprivate`, a type declared inside a function, a nested type outside its outer type.
   - Generic parameters of the enclosing header bind the name.
   - Repro:
     ```swift
     // Source/MultipartFormData.swift
     open class MultipartFormData {
         enum EncodingCharacters { static let crlf = "\r\n" }
         func f() -> String { EncodingCharacters.crlf }      // d on EncodingCharacters
     }
     // Tests/MultipartFormDataTests.swift
     enum EncodingCharacters { static let crlf = "\r\n" }
     ```
     Today: a picker of 2. Expected: the nested enum on line 3.
5. **Swift: `.case` and `Type.member` read enum cases and static members; a bare name in a type body reads that type first**
   - Repro:
     ```swift
     struct Endpoint {
         enum Path { case bytes(count: Int) }
         static func bytes(_ n: Int) -> Endpoint { Endpoint(path: .bytes(count: n)) }
         let path: Path
     }
     struct Stats { let bytes: Int }
     switch endpoint.path { case let .bytes(count): print(count) }   // d on bytes
     ```
     Today: a picker of 3, and more with any `let bytes` local. Expected: `case bytes(count: Int)`.
   - The same type-first rule makes `lock()` inside `extension Lock` a proven jump to `Lock.lock`.
