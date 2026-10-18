# Proposed `d` bench rows

Cursors and answers from the `d` pass of 2026-10-18 for languages and cases the bench does not
score yet (#1091). They are in the bench's formats (`cursors/LANG.tsv`, `answers/LANG.tsv`, see
`../README.md`) but keep the pass's scratch ids; a row renumbers them `LANGNNNN` and adds its
project to `projects.tsv`. Nothing here is read by `run`.

Answers outside a project are written with a placeholder instead of the recording machine's path:

| placeholder | stands for |
|---|---|
| `<sdk:Module>:LINE` | a Swift SDK declaration, in sourcekit-lsp's generated interface `Module.swiftinterface` (a temporary file; the line depends on the SDK) |
| `<dart-sdk>/` | the Dart SDK's root (3.13.5) |
| `<pub-cache>/` | the pub cache dio's packages were fetched into |
| `<typescript>/` | the `typescript` package of the servers' `node_modules` |
| `<cargo-registry>/` | `~/.cargo/registry/src/index.crates.io-*/` |
| `<python-stdlib>/` | CPython 3.11's `lib/python3.11/` (uv's build) |
| `<goroot>/` | GOROOT of Go 1.27.1 |

## dart

dio (github.com/cfug/dio) @ 45a2849a601b2137b797e4f183ee2da6b13c05a8, install `cd dio && dart pub
get`. `cursors-dart.tsv`: 104 cursors in `dio/lib`, drawn by `record.py`'s sampler with the Dart
`SPEC` and `KW` that `dart_bench.py` adds; `cursors-dart-interp.tsv`: 10 hand-picked inside string
interpolation. `answers-dart.tsv`: all 114 from `dart language-server` (Dart SDK 3.13.5), 24 of
them in the SDK. `dart_bench.py` drives `record.py` as it is at 0e47807c; #1106 changes the
signatures of `sample` and `oracle`, so it needs a small update once that lands.
Unchecked: one project, all of it library code. merl's answers depend on `dart` on `PATH`, since it
adds the SDK's `lib/` to the roots: 40 direct with it, 36 without.

## sql

lemmy (github.com/LemmyNet/lemmy) @ fc640cf86a130bc897d83dbd06d84c3c7d9b16e3. `cursors-sql.tsv`
and `final-answers-sql.tsv`: 110 cursors in the migrations, judged by reading. Objects, aliases and
renames were checked by hand. Unchecked: the 47 column targets were proposed by a script (the
latest `CREATE` or `RENAME` before the cursor) and not checked line by line, and 2 rows are
`skip: not checked`.

## java

halo @ fe77aacf894c9200ae4b4ddb0f904346813732d0, judged by reading the code at the cursor and at
the answer, in one pass, with no server. `cursors-java.tsv` + `answers-java.tsv`: 71 sampled;
`cursors-java-getter.tsv` + `answers-java-getter.tsv`: 40 getter chains (`x.getSpec().getY()`),
25 of them answered; `cursors-java-witness.tsv` + `answers-java-witness.tsv`: 12 type-witness
calls. Unchecked: every answer was read once; read it again before it joins `answers/`.

## kotlin

nowinandroid @ a49ed253d75e61a2b6ab80a8da677b57437b08eb. `cursors-kotlin.tsv` +
`answers-kotlin.tsv`: 71, judged by reading in one pass, as for Java, with the same caveat.

## csharp

eShop @ b4a40872005d4bb29e5b1fa1ff7e244143d39215, judged by reading; no server checked the answers.
`cursors-csharp.tsv` + `answers-csharp.tsv`: 113, of which 68 drawn by the bench's sampler
(`cursors-csharp-raw.tsv`, the same cursors before renumbering) and 45 member accesses on locals,
fields and parameters. `cursors-csharp-records.tsv` + `answers-csharp-records.tsv`: 6 record
properties. `cursors-csharp-nextline.tsv`: 4 chains continued on the next line, answered in
`sight-answers.tsv`. `sight-cursors.tsv` + `sight-answers.tsv`: those 123 `.cs` cursors in one
file. `cursors-csharp-razor.tsv` + `answers-csharp-razor.tsv`: 12 in `.razor` files, kept apart
since merl has no language for `.razor`.

## swift

Alamofire @ bda9ed57d72988a3a2ada33d824583541f86eac6, answers from sourcekit-lsp through
`record.py`'s client. `byname.tsv` + `byname-answers.tsv`: 353 member accesses in `Source/` where
merl jumps "by name, 1 match"; `implicit-sel.tsv` + `implicit-sel-answers.tsv`: 110 implicit
members (`.success(…)`, `.allow`). 165 answers are in the SDK (`<sdk:…>`): make them `skip`, as
the bench does for SDK answers, or the verdict of #1092. Unchecked: the cursors were picked by
merl's own outcome, not sampled.

## js

eslint @ 93de066d4125f8df013d40305e8009f4634a5cba. `cursors-js-thisclass.tsv` +
`answers-js-thisclass.tsv`: 94, every `this.x` in `lib/`'s `module.exports = class` files, from
tsserver. `cursors-js-commalist.tsv` + `answers-js-commalist.tsv`: 33 uses of comma-list
declarations, read from the code. `cursors-js-members.tsv`: 200 sampled members; the 26 merl jumped
on are `cursors-js-jumps.tsv`, with tsserver's answers in `answers-js-jumps.tsv`.
`cursors-js-extra.tsv`: 6 more, no answers; `cursors-js-all.tsv` is members and extra together.
Unchecked: a second JS project; 4 of the 26 jumps differ from tsserver (one is the CommonJS
convention the bench marks `skip`, two are plausible where tsserver had no answer).

## ts

outline @ 179d7dd7be2cb395d45f0151b858e14009cec9d1. `cursors-ts-members.tsv`: 200 sampled members;
the 35 merl jumped on are `cursors-ts-jumps.tsv`, with tsserver's answers in `answers-ts-jumps.tsv`.
Unchecked: the jumps through an import were not checked one by one.

## php

koel @ 2b4c6338f6e286d3cfc484e99be4c279b2c7b645. `cursors-php-property.tsv` +
`answers-php-property.tsv`: 21 Eloquent `@property` chains, read from the code.
`cursors-php-laravel.tsv`: 160 Laravel calls (facades, helpers, Eloquent, receivers typed with
Illuminate classes), no answers: they need `vendor/` (#1091).

## rust

ripgrep @ 3fce3b5bb0236da2df6d99672afb8a719642eca7. `xc-cursors.tsv` + `xc-answers.tsv`: 20
receivers typed by another crate's struct, from rust-analyzer. `fd-cursors.tsv`: 25 struct field
declarations, on the declaration by construction, so no answers. Unchecked: no second project.

## ruby

mastodon @ a4ac5f5670942265804dbb261713e1ff0109a402, all judged by reading: no Ruby server, gems
not installed. `cursors-ruby.tsv` + `answers-ruby.tsv`: 118. `cursors-ruby-bare.tsv`: 2,836 bare
calls in `app/`; `bare-byname-judged.tsv` holds the verdicts on the 122 where merl jumped by name.
Unchecked: the judged answers were never scored through the probe.

## python

paperless-ngx @ 6a5d06ee26a4620e6c8452870f157998d5b35050. `cursors-python.tsv` +
`answers-python.pyright.tsv`: 55, from pyright through `record.py`, 30 with a target.
Unchecked: pyright left 25 unanswered and was not asked again after a warm-up.

## go

gitea @ bea6fcaa840f2e14030aa2adc360969a1c8a0e0c (in `projects.tsv` as `vue`; a Go row needs its
own line) and caddy @ ea1e94b89972b83b189cbb946352abd9acec0f8c, answers from gopls with
`GOPROXY=off` (`answers-go-*.gopls.tsv`). `cursors-go-gitea.tsv`: 54 from `record.py`'s sampler;
`cursors-go-index.tsv`: 16 members after an index in gitea (`m[k].Get()`);
`cursors-go-elided.tsv`: 14 keys of caddy's table-test literals whose type is left out;
`cursors-go-caddy.tsv`: 16 local names in caddy. Unchecked: gitea's modules were not downloaded,
so gopls answered its in-project definitions only.
