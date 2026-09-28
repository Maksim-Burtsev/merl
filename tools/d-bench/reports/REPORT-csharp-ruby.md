# `d` in C#: eShop, judged (no oracle)

I judged all 80 cursors by reading the code. On top of that I ran 300 random `x.Member` cursors (generated protos and migrations excluded) and judged all 35 of their jumps. The repros were run with the bench binary.

## Tally

| bucket | rows | ok | pick-hit | WRONG | pick-miss | none |
|---|---|---|---|---|---|---|
| project | 27 | 10 | 14 | 0 | 2 | 1 |
| external (BCL, ASP.NET, EF Core, MAUI, gRPC, MSTest, Duende…) | 32 | – | – | 1 | 3 (pickers of project namesakes) | 28 (honest) |
| artifact | 21 | 9 on a declaration, 11 namespace segments (`using`, `namespace`, `global::`), 1 keyword |

- **Artifact share by merl's outcome:** jump 1/12, picker 8/27, stay 12/41.
- **Project rows:** 37% jump right, 52% get a picker that holds the answer, and there is no wrong jump.
- **External rows:** 32 of the 59 non-artifact cursors are framework calls. For 28 of them "no definition" is the honest answer, because C# has no source to search.
- **Member probe:** 35 jumps gave 26 ok, 8 WRONG (24%) and 1 artifact. All 8 WRONG are framework members that landed on a project namesake. The 80-row sample shows this risk only once.

## Causes, most harmful first

| # | cause | count (sample; probe) | WRONG? | example | fix sketch | risk | size |
|---|---|---|---|---|---|---|---|
| C2 | A framework member lands on a project namesake. The by-name search has no roots, so a unique project declaration is jumped to. | 1 WRONG + 3 pickers; probe: 3 WRONG caught here (others under C1, C4, C7) | yes | `eShop.ServiceDefaults/AuthenticationExtensions.cs:31` `JsonWebTokenHandler.DefaultInboundClaimTypeMap.Remove("sub")` → `tests/ClientApp.UnitTests/Mocks/MockSettingsService.cs:185` `void Remove(string key)`. Should be none (`Dictionary.Remove`). Probe: `Task.Delay(Delay)` → `public int Delay`; `HttpStatusCode.Created` → `GrantViewModel.Created`. | (a) A `private` member (explicit, or a class/struct member with no access modifier) is reachable only from its own type: drop it unless the open file declares that type (a `partial` part counts). (b) A capitalised qualifier the project declares nowhere, not even as a namespace prefix, is a type from outside, so its member is too: "no definition". | (a) none. (b) A project type the rules miss would lose its by-name guess, so fire only when `def_patterns(qualifier)` finds nothing. | S |
| C1 | No scope for locals and parameters (bindings exist only for Python/TS/Go). The member rule also matches `var x =` / `T x =`, so every local of every method is a candidate, even behind a dot. | 4 (0001, 0040 pick-miss; 0021 none; 0079 pick-hit of 9); probe 2 WRONG | yes | `Ordering.API/Extensions/BasketItemExtensions.cs:18` `item.ProductName` → picker of 10 `var item` in 6 other files. Should be the parameter `this BasketItem item` (:13). Probe: `view.GetValue(…)` → `WebApp/Components/Chatbot/ChatState.cs:113`, a local function in another project. Repro `locals/`: parameter `options` plus one `var options` elsewhere → WRONG jump. | (a) A declaration whose enclosing declaration (the `qualified` walk) is a method, constructor, accessor or lambda is a local. It is a candidate only when the cursor is inside that method, and never behind a dot. (b) Run the brace-scoped `block_bindings` (as for TS/Go) on C#: method, constructor, primary-constructor and lambda parameters (`x =>`, `(a, b) =>`), `foreach (var x in`, `catch (E e)`, `using var x`, `out var x`, `is T x`, `var x =` above the cursor → `Local`. | low; forms it can't read (`var (a, b)`) fall to "no definition", not to a namesake | S (a), M (b) |
| C4 | Every project of the solution is searched, but a file only sees its own `.csproj` and the projects it references. | 5 pick-hits carry another project's namesake (0029, 0033, 0035, 0056, 0062); probe 1 WRONG | yes | Probe: `ClientApp/Controls/ToggleButton.cs:96` `_toggleImage.Source` → `Ordering.API/Extensions/LinqSelectExtensions.cs:41`. `OrderAggregateTest.cs:168` `new Address(…)` offers `ClientApp/Models/User/Address.cs`, although `Ordering.UnitTests.csproj` does not reference ClientApp. | Take the nearest `*.csproj` above the open file and follow `<ProjectReference Include="..\X\X.csproj"/>` transitively. Drop candidates under any `.csproj` directory outside that set. | Refuse to filter when the csproj has `<Compile Include="..`, imports a `.projitems`, or no csproj exists. | M |
| C3 | A type position offers non-types: properties named like the type (`public Buyer Buyer`), fields, and the class's own constructors. | 5 pick-hits (0029, 0031, 0033, 0045, 0056) | no | `IBuyerRepository.cs:9` `Buyer Update(Buyer buyer)` → picker of 7: the class, 2 constructors, 4 properties named `Buyer` (2 in other projects). Should be `Buyer.cs:5`. | The word is a type when a variable name, `<`, `[]` or `?` follows it, or when `new `, `: `, `typeof(`, `is `, `as `, `(T)` or a generic argument list precedes it. Then only type rules count, and a constructor in the file of a same-named type folds into that type. | low | S |
| C5 | Members in an object initializer are looked up by name. | 2 pick-hits (0035, 0062) | no | `Identity.API/Quickstart/Device/DeviceController.cs:200` `Checked = …` inside `new ScopeViewModel {` → picker of 3. Should be `Quickstart/Consent/ScopeViewModel.cs:14` (same namespace). | Scan back to the `{` that opens the initializer and read `new T` / `new T(…)` before it (or the declared type of a target-typed `new()`). Look the member up in T and its project bases. | low | M |
| C6 | The enclosing type is not preferred for a bare name. C# binds a simple name to a local first, then to a member of the enclosing type. | 1 pick-hit (0030) | no | `…WhenOrderStartedDomainEventHandler.cs:47` `_buyerRepository` → the same field in 6 handler classes. Should be the same file, line 7. | A bare, non-local word declared by the enclosing class, one of its `partial` parts, or a project base named in its header → that declaration. | low | S |
| C7 | No typed receivers for C#, although C# writes its types everywhere. | 4 pick-hits (0004, 0039, 0051, 0067); probe 2 WRONG | yes (probe) | `tests/Application.UnitTests/IdentityConfigurationTests.cs:49` `service.ExtractRedirectUriFromReturnUrl` with `var service = new RedirectService();` → interface + class. Probe: `_badgeIndicator.Text` (a MAUI `Label` field) → `BadgeView.Text`. | Port `typed.rs`: read the type from `var x = new T()`, `T x`, fields, properties, parameters, primary-constructor parameters, and a method's declared return type (`Task<T>` under `await`). Look the member up in T and its `: Base, IFace`. If T is outside the project, answer "no definition", but first check extension methods `static … M(this T x`. | medium (generics, extension methods) | L |
| C9 | Namespace segments are read as names and matched by their last part only. | 4 artifact pickers (e.g. 22 `namespace *.Migrations` rows for `Microsoft.EntityFrameworkCore.Migrations`); probe 1 WRONG | yes (probe) | Probe: `Identity.API/GlobalUsings.cs:41` `global using Microsoft.Extensions.Options;` → the property `CatalogServices.Options`. | On a `using` or `namespace` line, and after `global::` / `alias::`, the word is a namespace. Match only `namespace` lines whose dotted name starts with the prefix up to the word. | none | S |
| C8 | Overload arity is not read. | 0017 (a correct picker of 2), 0020 (static `object.Equals(a, b)` → 16 one-parameter `Equals`) | no | `ClientApp/Services/Basket/Protos/Basket.cs:115` `return Equals(_unknownFields, other._unknownFields);` | Drop candidates whose parameter count can't take the call's argument count (defaults and `params` make it a range). | low | S |
| C10 | A constructor is not recognised when its parameters wrap (`public BasketViewModel(` over several lines) or its body opens on the same line (`public Buyer(string id) { }`). | 0024 (artifact), repro `typepos/` | no | – | Allow the line to end after `(`, and allow `{ … }` after `)`. | low | S |

## Why 14 rows got a picker instead of a jump, and what narrows them

| rows | why a picker | narrowed by |
|---|---|---|
| 0029, 0031, 0033, 0045, 0056 | the type plus its constructors, properties named like it, a namesake in another project | C3 + C4 → jump |
| 0035, 0062 | an initializer member, with namesakes in other types and projects | C5 + C4 (0035 also needs a same-namespace preference) |
| 0030, 0079 | a field or local of the same name in other classes and methods | C6, C1 |
| 0004, 0039, 0051, 0067 | a member on a value: interface + implementation, or two types | C7 |
| 0017 | two overloads with the same arity | nothing; a picker is right here |

## Latency

No row over ~35 ms. The sample median is 10 ms (p90 18); the member probe median is 17 ms (max 34). A member lookup greps the same pattern twice: once for the `Outer.x` path attempt, then again for the search by name. Reusing the first grep's hits would halve it.

## Top improvements

**1. `d` (C#): a framework member never lands on a project namesake** (C2 + C1a, S)
Drop three kinds of candidate:
- a `private` member outside the open file's type;
- a declaration nested in a method (a local or a local function) when the cursor is outside that method or behind a dot;
- every member of a capitalised qualifier the project declares nowhere.

This fixes 0034 and 5 of the 8 probe WRONGs; C4 takes a sixth and C7 the last two.
Repro (`repro/ext`, `repro/localfn`):
```csharp
// Mock.cs
class MockSettings { void Remove(string key) { } }        // private
// Startup.cs, cursor on Remove → today: jumps to Mock.cs:3; want: no definition
JsonWebTokenHandler.DefaultInboundClaimTypeMap.Remove("sub");
// Tabs.cs, cursor on GetValue → today: jumps to a local function inside ChatState.Load
string Badge(BindableObject view) => (string)view.GetValue(BadgeTextProperty);
```

**2. `d` (C#): locals and parameters resolve in their own method** (C1b, M)
Run `block_bindings` for C#: parameters of the enclosing method, constructor, primary constructor or lambda; `foreach`, `catch`, `using`, `out var` and `is T x` bindings; `var x =` / `T x =` above the cursor. Answer `Local`, and never offer another method's local. Fixes 0001, 0021, 0040 and 0079.
Repro (`repro/locals`), cursor on `options` in `B` → today: jumps to `Other.C.options`:
```csharp
public void B(Options options) { options.Enabled = true; }   // Tests.cs
public void C() { var options = new Options(); }              // Other.cs
```

**3. `d` (C#): a file sees its own project and the ones its .csproj references** (C4, M)
Read the nearest `*.csproj` of the open file and its `ProjectReference`s, transitively; drop candidates under any other project directory. Refuse when the project compiles files from outside it or no csproj exists. This removes other-project namesakes from 5 pick-hits and fixes one probe WRONG.
Repro (`repro/proj`): `Shop.Api/Address.cs` and `Shop.App/Address.cs` both declare `Address`. In `Shop.App/Page.cs`, `new Address { … }` with the cursor on `Address` → today: a picker of both; want: `Shop.App/Address.cs`.

**4. `d` (C#): types in type positions, initializer members in the constructed type, bare names in the enclosing class** (C3 + C5 + C6, S/M)
- A word followed by a variable name, or preceded by `new`/`:`/`typeof(`/`is`/`as`, finds only type declarations, and a constructor folds into its type.
- A `Name = …` inside `new T {` is looked up in T.
- A bare name the enclosing class declares resolves to that declaration.

Together with issue 3, 7 of the 14 sample pick-hits become jumps (4 without it).
Repro (`repro/typepos`): `Buyer Update(Buyer buyer);`, with both `class Buyer` and `public Buyer Buyer { get; set; }` in the project, cursor on the parameter type → today: a picker (property + class); want: the class.

**5. `d` (C#): typed receivers** (C7, L)
Port the Python/TS/Go chain to C#'s explicit types:
- locals from `new T()` / `T x`, fields, properties, parameters, primary-constructor parameters;
- a method's declared return type (`Task<T>` unwrapped after `await`);
- the member in T and its bases;
- T outside the project → "no definition", after checking extension methods.

Fixes 4 pick-hits and 2 probe WRONGs.
Repro: `var service = new RedirectService(); service.ExtractRedirectUriFromReturnUrl(u);` with the method declared in both `IRedirectService` and `RedirectService`, cursor on the method → today: a picker of 2; want: `RedirectService`.

---

# `d` in Ruby: mastodon, judged (no oracle)

I judged all 80 cursors by reading the code. On top of that I ran 300 random `x.member` cursors (`spec/` and `db/migrate/` excluded) and judged all 45 of their jumps. Repros are in `work-ruby/repro/` and `work-ruby/repro-bt/`.

## Tally

| bucket | rows | ok | pick-hit | WRONG | pick-miss | none |
|---|---|---|---|---|---|---|
| project | 33 | 17 | 6 | 2 | 6 | 2 |
| external (core, stdlib, gems) | 40 | – | – | 4 | 10 (+1 acceptable picker of the project's reopenings of `module ActiveRecord`) | 25 (honest) |
| artifact | 7 | 3 words in SQL heredocs, 1 in a `%i()` literal, 2 on a declaration, 1 sampler column counted in characters on a non-ASCII line ("no word") |

- **Artifact share by merl's outcome:** jump 0/23, picker 1/24, stay 6/33.
- **Project rows:** 52% ok.
- **External rows:** 4 of the 15 external rows where merl answered anything are WRONG jumps.
- **Member probe:** 45 jumps gave 13 ok, 31 WRONG (70%) and 1 artifact. A Ruby member jump is more often wrong than right.

## Causes, most harmful first

| # | cause | count (sample; probe) | WRONG? | example | fix sketch | risk | size |
|---|---|---|---|---|---|---|---|
| R1 | A method found by name in the project alone is jumped to, though the receiver is unknown and nothing outside the project is searched (`each`, `first`, `empty?`, `include?`, `info`, `configure`, `normalize`, `use`). | 2 WRONG (0015, 0024) + 1 pick-miss (0049); probe 15 WRONG (+2 shared with R3) | yes | `lib/vite/builder.rb:21` `logger.info { … }` → `app/models/worker_batch.rb:85` `def info`. Probe: 4 × `x.each` → `app/models/trends/history.rb:89`. | (a) For a kind with no roots, a member on a value found only by name is offered, never jumped to. merl already offers a single candidate when a search is cut short. (b) Gem roots (see below) make the lone project `def info` one of several. | (a) A correct unique jump costs one more Enter; the probe has 13 ok against 31 WRONG. | S (a), M (b) |
| R2 | Assignments declare names outside their scope. The rule `^\s*@{0,2}w\s*(\|\|)?=` makes every local `w =` and every `@w =` in the project a declaration of `w`, behind a dot too, and treats `@w` and `w` as one word. | 2 WRONG (0000, 0076), 7 pick-miss (0005, 0040, 0041, 0061, 0064, 0071, 0075), noise in 0007, 0044, 0068 (275 rows); probe 8 WRONG | yes | `config/initializers/open_uri_redirection.rb:7` `uri2.scheme` → `lib/sanitize_ext/sanitize_config.rb:50` `scheme = if …`, a local of another file. `app/workers/scheduler/scheduled_statuses_scheduler.rb:18` block parameter `scheduled_status` → a local of `PublishScheduledStatusWorker#perform`. Probe: `poll.votes` → `@votes = []` in `app/services/vote_service.rb:17`; should be `app/models/poll.rb:31` `has_many :votes`. | A local `w =` is a candidate only between the cursor's enclosing `def` and its `end`. `@w =` counts only for the word read with its `@`, and only in the cursor's class. Behind a dot, only `def`, `attr_*`, `alias` and DSL lines count. When none match, don't fall back to the assignment rule (the `members_by_name … unwrap_or_else(project_definitions)` fallback in `definition.rs`). | low | S |
| R4 | `Const.meth` accepts instance methods. The path rule keeps any `def meth` whose qualified name is `Const.meth`, and `class << self` breaks the walk, so the class methods inside it are unqualified. The status line says `via Const`, so the wrong jump reads as proven. | probe 3 WRONG; sample: 3 pickers of other classes' `find` (0019, 0054, 0078) | yes | `app/serializers/concerns/notification_fallback_concern.rb:47` `TextFormatter.link_to_mention(account)` → `app/lib/text_formatter.rb:124`, the instance method, "(via TextFormatter)". Should be :80, inside `class << self`. | For a constant qualifier, count only singleton declarations: `def self.m`, `def Const.m`, a `def` inside `class << self` (read as the class around it), `scope :m`, `class_methods do` / `module ClassMethods` of an included concern, `module_function`. Refuse an instance `def m`. If nothing is found, answer "no definition" (it is an ActiveRecord class method), not a by-name search over other classes. | low | S/M |
| R3 | `?`, `!` and `=` are cut off the word, so `remote` finds `def remote?` and `fetch` finds `def fetch?` and `def fetch!`. In Ruby these are different methods. | 2 pickers (0012, 0073); probe 4 WRONG | yes | Probe: `app/models/report_filter.rb:71` `Account.remote` → `app/models/account.rb:203` `def remote?` "(via Account)". Should be `account.rb:146` `scope :remote`. | Read the suffix at the cursor. `w?` finds only `def w?`; a bare `w` finds a `def w` not followed by `?`, `!` or `=`; `x.w = v` finds `def w=` or `attr_writer`/`attr_accessor :w`. | none | S |
| R5 | `class A::B` counts as a declaration of `A`, and `A::B` is not read as a path. Ruby's separator is set to `.`, so the path rule never sees `::`. | 1 WRONG (0048), 2 pick-miss with 140/144 rows (0027, 0045), 2 pick-hits (0060: 34 rows; 0017: a superclass reported as "at a declaration") | yes | `config/initializers/rack_attack.rb:69` `Rack::Attack.blocklist`, cursor on `Rack` → line 5 `class Rack::Attack`. Should be the rack gem's `module Rack`. `ActivityPub` is a Zeitwerk namespace with no declaration, yet gets a picker of 140 `class ActivityPub::X` lines. | Pattern `(?:[\w:]+::)?{w}\b(?!::)`. On a `class` line, the word right of `<` is a use, never "at a declaration". For `A::B::w`, keep declarations whose qualified name ends in `A.B.w` (compact and nested forms). | none | S |
| R0 | Ruby literals are read with the C family's forms: `#` is not a comment, a backtick opens a template that runs to the next backtick, `//` and `/* */` are comments, and heredocs are code. | sample 0 (3 artifacts sit in SQL heredocs); found by probing: 3 "no definition" in `user.rb`, a WRONG in a repro, probe m0097 | yes (repro) | `app/models/user.rb:337` "# … since \`update_all\`\`" has an odd number of backticks and hides every declaration below it: `d` on `prepare_new_user!` (:247) says "no definition", though the def is at :479. In `repro-bt/`, the same bug makes `prepare!` jump to another class's namesake. | Give Ruby its own forms in `literal_lines`: `#` comments, `=begin`/`=end`, heredocs `<<~ID` / `<<-ID` / `<<ID` up to the line holding `ID`. No backtick templates, no `//`, no `/* */`. | none (3 of 3,266 files have an odd backtick, but `User` is the central model) | S |
| R6 | Rails DSL and the schema have no rule: scopes, associations, columns, `remotable_attachment`, route helpers. | 2 none (0014, 0056), 2 pick-miss (0052, 0079); probe 6 WRONG (shared with R2–R4) | yes | `app/services/activitypub/fetch_featured_collection_service.rb:84` `…local.without_suspended.first` → none. Should be `app/models/concerns/account/suspensions.rb:9` `scope :without_suspended`. `app/serializers/activitypub/featured_collection_serializer.rb:29` `object.language` → 8 unrelated lines. Should be `db/schema.rb:406` `t.string "language"` of `collections`. | DSL lines: `^\s*(scope\|has_many\|has_one\|belongs_to\|has_and_belongs_to_many\|has_attached_file\|attribute\|enum)\s+:w\b` and `delegate … :w … to:`. Columns: `^\s*t\.\w+\s+"w"` in `db/schema.rb`, one row per table. Route helpers `w_path` / `w_url` → `as: :w` in `config/routes.rb`; paths nested inside `resources`/`namespace` are beyond rules. | low (these lines are declarations by construction) | S (DSL, schema), M (routes) |
| R7 | A keyword-argument label or a hash key is looked up as a name. | 1 WRONG (0070) | yes | `app/services/activitypub/process_status_update_service.rb:419` `snapshot!(…, rate_limit: false)` → `app/models/concerns/rate_limitable.rb:17` `def rate_limit`. Should be the keyword parameter of `build_snapshot` (`snapshot_concern.rb:14`), reached through `snapshot!(**)`, or no definition. | A word followed by `:` and a space (not `::`), inside call parentheses or `{ }`, is a label: answer "no definition" or only offer candidates, as Python's `keyword_argument` rule does. | none | S |
| R8 | No scope walk for parameters, block parameters and locals. | 3 (0061 and 0071 pick-miss, 0076 WRONG, which is also R2) | via R2 | `app/controllers/auth/sessions_controller.rb:145` `user.update_sign_in!` → 68 `user =` lines. Should be the parameter on :139. | Ruby bindings: `def m(a, b = 1, *c, d:, **e, &f)`, `do \|x, (y, z)\|`, `{ \|x\| }`, and locals above in the same def or block → `Local`. | low | M |
| R9 | A bare call does not prefer the cursor's class. | 1 pick-hit (0001) | no | `app/models/account_conversation.rb:88` `participants_from_status(…)` → itself at :107 plus a migration's copy. | Answer a bare call from the enclosing class or file first, then from the modules it `include`s, before the search by name. | low | S |

Of the probe's 31 WRONG jumps, the S rules for R2, R3, R4 and R6 fix 15. The other 16 are core or gem methods that happen to have one project namesake (R1).

## External calls: would reading the installed gems reach them?

**What the 40 external targets are**
- **29 are Ruby source in a gem or the stdlib:**
  - activerecord 8, railties 3, activesupport 2, parslet 2, sidekiq 2, i18n 2;
  - one each from rack, rack-attack, json-ld, actionpack, simple-navigation, addressable, redis, devise;
  - the stdlib's `uri`, `logger` and `openssl`.

  Devise's `mattr_accessor` would need a rule of its own.
- **8 are core classes written in C:** `Hash#fetch`, `Kernel#Rational`, `#send`, `Object#freeze`, `ENV`, `Errno`, `Enumerable#take`. There is no `.rb` source for them anywhere. Only the RBS signatures shipped by the `rbs` gem (`core/*.rbs`, bundled with Ruby ≥ 3.0) declare them, the way `.pyi` or `.d.ts` stubs do.
- **3 are dynamic** and have no declaring line anywhere: `tag.samp` (method_missing), `config.action_controller.asset_host` (OrderedOptions), and an aws-sdk keyword.

**On this machine, nothing would be reached**
- Only the system Ruby 2.6.10 is installed. mastodon wants Ruby 4.0.7 and bundler 4.0.20, so `bundle show --paths` fails and none of the gems are present.
- `gem env` lists 2.6's directories, which hold none of them.

**How to read the gems without running project code**
- `bundle show --paths` evaluates the project's `Gemfile`, which is Ruby code, and #183 rules that out.
- Instead, read `Gemfile.lock` (`name (version)` lines under `specs:`) and look for `<gemdir>/gems/<name>-<version>/` in these places:
  - the `BUNDLE_PATH` from `.bundle/config` (`vendor/bundle/ruby/<abi>/`);
  - `GEM_HOME` / `GEM_PATH`;
  - the version manager's directory for the `.ruby-version` Ruby (`~/.rbenv/versions/X`, `~/.local/share/mise/installs/ruby/X`, `~/.rubies/ruby-X`);
  - `gem env gempath`, run from `/`.
- Git gems live under `bundler/gems/<name>-<sha>`, and the stdlib under `lib/ruby/<abi>`.

**Reach is not precision.** `find` would become a picker of every gem's `def find`. The gain is that a lone project namesake stops looking unique (R1), and `Rack`, `I18n` and `Sidekiq::Worker` get a real answer.

## Why 6 rows got a picker instead of a jump, and what narrows them

- 0068: local `status`, 275 rows with the right one first; 0044: `link` plus locals → R2 / R8.
- 0060: `Account`, 34 `module Account::X` rows; 0017: a superclass → R5.
- 0073: `following?`, 8 rows including `def following` → R3 leaves one row.
- 0001: a bare call → R9.

## Latency

- **Bare lookups:** 42–46 ms over 3,266 `.rb` files.
- **Member lookups:** 86–94 ms (probe p90 102, max 139).
- **First lookup of a fresh App:** 165–180 ms.

Nothing comes near 300 ms. The member cost is the same pattern grepped twice: once for the `Const.meth` path attempt, then for the search by name. Reusing the first grep's hits halves it.

## Top improvements

**1. `d` (Ruby): a local or an `@ivar` is declared only in its own method and class, never behind a dot** (R2, S)
- Keep local assignments only inside the cursor's enclosing `def…end`.
- Read `@x` with its sigil, and look it up only in the cursor's class.
- Behind a dot, count only `def`, `attr_*`, `alias` and DSL lines, and when nothing matches, don't fall back to the assignment rule.

Fixes 0000 and 0076, 8 probe WRONGs, and cuts 0068 from 275 rows to 1.
Repro (`repro/app`), cursor on the second `scheme`:
```ruby
def self.redirectable?(uri1, uri2) = uri1.scheme.casecmp(uri2.scheme)  # redirect.rb → today: jumps to
def transform(node); scheme = node['href']; end                        # sanitize.rb:3, a local
```

**2. `d` (Ruby): comments, heredocs and `=begin` are read as Ruby writes them** (R0, S)
Ruby's `literal_lines` currently uses the C family's forms. Give it `#` comments, `=begin`/`=end` and `<<~ID` heredocs, and drop backtick templates and `//` / `/* */`. This fixes "no definition" for every method below `app/models/user.rb:337`.
Repro (`repro-bt`): in `user.rb`, `# since \`update_all\`\`` above `def revoke!; prepare!; end` and `def prepare!`. Cursor on `prepare!` → today: jumps to `Session#prepare!` in `user2.rb`; want: `user.rb:8`.

**3. `d` (Ruby): `?`, `!` and `=` belong to the name, `Const.m` is a singleton method, and `class A::B` declares `B`** (R3 + R4 + R5, S/M)
- Match the method-name suffix exactly.
- For a constant qualifier, only `def self.m`, `class << self`, `scope` and `class_methods` count.
- The class pattern requires the word to be the last segment, and `A::B` is used as a path.

This removes 7 probe WRONGs and the "via" wrong jumps, and 0048.
Repro (`repro/app`):
- `Account.remote`, with both `scope :remote` and `def remote?` in `Account` → today: `def remote?` "(via Account)".
- `TextFormatter.link_to_mention(a)` → today: the instance method on :8; want: :3, inside `class << self`.
- `Rack::Attack.blocklist`, cursor on `Rack` → today: `class Rack::Attack`; want: no definition.

**4. `d` (Ruby): Rails declarations: `scope`, associations, `db/schema.rb` columns** (R6, S)
Add DSL lines (`scope`, `has_many`, `has_one`, `belongs_to`, `has_attached_file`, `attribute`, `enum`, `delegate … to:`) as declarations, and `t.<type> "w"` in `db/schema.rb` as a column. This fixes 0056 and the probe's `votes`, `inbox_url`, `in_reply_to_account_id` and `signed_in_recently`, and gives 0052 and 0079 their real answer.
Repro (`repro/app`): `has_many :followers` in `Account`, and `account.followers.first` elsewhere, cursor on `followers` → today: "no definition"; want: the `has_many` line.

**5. `d` (Ruby): a member found only by name, with nothing outside searched, is offered rather than jumped to; then search the installed gems** (R1, S then M)
Until `Gemfile.lock` plus the gem directories and the RBS core signatures are read, a unique project match for `x.m` is offered, not jumped to. This covers the 16 WRONGs the other rules leave, and 0015 and 0024.
Repro (`repro/app`): `class WorkerBatch; def info; {}; end; end` and `logger.info { 'x' }`, cursor on `info` → today: "info → WorkerBatch.info (by name, 1 match)", a jump.
