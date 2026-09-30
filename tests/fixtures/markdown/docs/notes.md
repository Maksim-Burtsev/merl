---
title: Notes
see: "[README](../README.md)"
#      ^ d: none
---

# Notes

An inline link, on its text and on its target:

See the [README](../README.md#languages) for languages.
#        ^ d: README.md:5
# status: link README.md#languages
See the [README](../README.md#languages) for languages.
#                             ^ d: README.md:5

With a title, in angle brackets, percent-encoded, from the root:

A [titled one](../README.md "The readme").
#  ^ d: README.md:1
# status: link README.md
An [angled one](<../README.md#setup>).
#   ^ d: README.md:13
An [encoded one](../README%2Emd#languages).
#   ^ d: README.md:5
A [rooted one](/README.md#languages).
#  ^ d: README.md:5

Anchors: backticks and punctuation, a release, two headings of one name, setext, an `<a id>`:

The [full one](../README.md#what-d-recognises-language-by-language).
#    ^ d: README.md:9
The [release](../README.md#080---2026-09-27).
#    ^ d: README.md:24
The [first setup](../README.md#setup).
#    ^ d: README.md:13
The [second setup](../README.md#setup-1).
#    ^ d: README.md:17
The [setext one](../README.md#setext-heading).
#    ^ d: README.md:21
The [legacy one](../README.md#legacy-anchor).
#    ^ d: README.md:26
A [heading here](#local-heading), in this file.
#  ^ d: docs/notes.md:123
# status: link #local-heading
A [line](../README.md#L7) and [lines](../README.md#L7-L9).
#  ^ d: README.md:7
# status: link README.md#L7
A [line](../README.md#L7) and [lines](../README.md#L7-L9).
#                              ^ d: README.md:7

Reference links, through the definition below, whose label ignores case:

A [full reference][Readme], a [collapsed one][] and a [readme] alone.
#  ^ d: README.md:13
A [full reference][Readme], a [collapsed one][] and a [readme] alone.
#                              ^ d: README.md:5
A [full reference][Readme], a [collapsed one][] and a [readme] alone.
#                                                      ^ d: README.md:13

[readme]: ../README.md#setup
#^ d: README.md:13

[collapsed one]: ../README.md#languages
#^ d: README.md:5

HTML, and an image, which is not followed:

<a href="../README.md#setup-1">the second setup</a>
#                                  ^ d: README.md:17
![diagram](../README.md)
# ^ d: none
# status: no link here

What is missing says so:

A [missing file](nope.md) and a [missing heading](../README.md#langs).
#          ^ d: none
# status: no file docs/nope.md
A [missing file](nope.md) and a [missing heading](../README.md#langs).
#                                        ^ d: none
# status: no heading #langs in README.md

A path in a code span, from the root or from here, with a line or not:

Open `src/shop/mod.rs`, `src/cart/mod.rs:3` or `src/cart/mod.rs:3:8`.
#         ^ d: src/shop/mod.rs:1
# status: path src/shop/mod.rs
Open `src/shop/mod.rs`, `src/cart/mod.rs:3` or `src/cart/mod.rs:3:8`.
#                            ^ d: src/cart/mod.rs:3
# status: path src/cart/mod.rs:3
Open `src/shop/mod.rs`, `src/cart/mod.rs:3` or `src/cart/mod.rs:3:8`.
#                                                   ^ d: src/cart/mod.rs:3
The `guide.md` beside this file, and `../README.md` above it.
#    ^ d: docs/guide.md:1
The `guide.md` beside this file, and `../README.md` above it.
#                                        ^ d: README.md:1
Two files carry `mod.rs`; `Basket` names none.
#                ^ d: picker src/shop/mod.rs:1, src/cart/mod.rs:1
# status: path mod.rs: 2 files
Two files carry `mod.rs`; `Basket` names none.
#                          ^ d: none
# status: no link here
Prose is no link.
#     ^ d: none
# status: no link here

Code and comments are text:

```markdown
See the [README](../README.md#languages).
#        ^ d: none
```

<!--
See the [README](../README.md#languages).
#        ^ d: none
-->

Inline <!-- [README](../README.md) --> too.
#            ^ d: none

## Local heading

The end.
