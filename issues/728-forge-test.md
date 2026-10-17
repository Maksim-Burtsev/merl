---
title: Forge test
tags: [gfm, glfm]
---

# Forge test

Every GFM and GLFM construct, one per section, to compare GitHub, GitLab and `p`.

## Links

See [the guide](docs/guide.md), [its setup part](docs/guide.md#setup) and [the keys below](#html).
Bare URL: https://example.com/path and www.example.com, mail: someone@example.com.
Autolink <https://example.com/angle>.

## HTML

<!-- a comment the forges hide -->

First line<br>second line after a br.

| Key | What |
|---|---|
| <kbd>Ctrl</kbd>+<kbd>D</kbd> | half a page down<br>keeps the cursor row |
| H<sub>2</sub>O, x<sup>2</sup> | sub and sup |

<details>
<summary>Click to expand</summary>

Hidden body with **bold** text.

</details>

<p align="center">
  <b>Centred bold</b> in a paragraph tag.
</p>

<a name="old-anchor"></a>
Text after a named anchor, <span title="tip">a span</span>, <ins>inserted</ins> and <del>deleted</del>.

## Emoji and math

Shortcode :rocket: and :+1:.

Inline math $E = mc^2$ and a block:

$$
\sum_{i=1}^n i = \frac{n(n+1)}{2}
$$

## Alerts and footnotes

> [!NOTE]
> An alert.

A footnote[^1].

[^1]: The note.

## GitLab only

[[_TOC_]]

Inline diff: {+ added +} and [- removed -].

Colour chip: `#FF0000`.

>>>
A multiline quote.
>>>

## GitHub only

![logo](logo.png#gh-dark-mode-only)
