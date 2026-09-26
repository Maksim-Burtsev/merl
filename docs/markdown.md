# Markdown, rendered

Agents write Markdown: plans, specs, reports, `AGENTS.md`, changelogs. `p` on a `.md` or
`.markdown` file shows it rendered, in place, in the code pane; `p` again shows the source. The
place is kept both ways: the preview opens on the block the source was showing, as far down the
pane, and the source comes back with the cursor on the line the preview was showing. Enter in the
preview goes to the source at that place and starts editing, as it does on the source.

A Markdown file opens as source, as any other file; `p` renders it until `p` again or until merl
quits, the way `w` flips wrapping for one file. Another Markdown file opens as source. The status
bar says `[preview]` where it says `[code]`, and `p` on a file that is not Markdown says
`not Markdown`.

| Markdown | In the preview |
|---|---|
| `#` … `######` headings | the text without `#`, bold, in the theme's heading colour; a rule under H1 and H2 |
| `**bold**`, `*italic*`, `~~strike~~` | bold, italic, crossed out |
| `` `code` `` | on a tint |
| fenced and indented code blocks | highlighted by the info string (`rust`, `py`, …) in the theme's colours, on a tint, no fences |
| `-` / `1.` lists, nested | `•` / numbers; a wrapped line goes on under the item's text |
| `- [ ]` / `- [x]` | `☐` / `☑` |
| `> quote`, GitHub alerts (`> [!NOTE]`) | a `│` bar; an alert's title and bar in its colour |
| tables | box drawing, each column aligned as its `:---`, `:---:` or `---:` says |
| `---` | a rule across the pane |
| `[text](url)` | the text, underlined in the link colour; the URL is in the source |
| `![alt](image.png)` | `▣` and the alt text |
| footnotes | the reference as `[1]`, the notes at the end under a rule |
| YAML front matter, raw HTML | as written, dim |

Images, heading sizes, Mermaid and math stay text: a terminal has no way to draw them.

The preview is rendered from the open buffer, not from the disk: an edit shows in it, and a file
an agent rewrites (merl reloads it) renders again. Prose reflows to the width of the pane, emoji
and CJK taking the columns they are drawn in. A table wider than the pane stays in one piece: its
widest columns give up columns first and their cells wrap. A long line of code wraps as it does in
the source, under its indent. The colours come from the theme, so `T` repaints the preview, code
blocks included.

The preview has a cursor row: Up / Down, PgUp / PgDn, Ctrl+D / Ctrl+U and Ctrl+Home / Ctrl+End
move it, and `{` / `}` go to the blank row before the previous or next block. The status bar and
the jump history follow the source line under it: reading adds no stop, Ctrl+Home, Ctrl+End and a
far `{` / `}` add one as in the source. `[` and `]`, `:`, `o`, `s`, `D`, `t`, `T`, `?` and `q` work
as everywhere, and Ctrl+C copies the source line. What acts on a word, a column
or a selection (`/`, `n`, `v`, `d`, `u`, Home / End, Shift or Alt with an arrow, `w`) has nothing
to act on in the preview and does nothing; `p` or Enter take you to the source for it.

In `--review` the preview keeps the diff in its gutter: the rows of lines the branch added have
the green bar and tint, a line that replaced one the blue bar a changed line gets, and a red mark
stands where the branch deleted lines. The deleted text itself is in the source. `c` / `C` walk
the hunks in either, the preview's rows one hunk further when the next is on the row already.
