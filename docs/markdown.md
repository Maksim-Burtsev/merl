# Markdown, rendered

Agents write Markdown: plans, specs, reports, `AGENTS.md`, changelogs. `p` on a `.md` or
`.markdown` file shows it rendered, in place, in the code pane; `p` again shows the source. The
place is kept both ways: the preview opens on the block the source was showing, as far down the
pane, and the source comes back with the cursor on the line the preview was showing. Enter in the
preview goes to the source at that place and starts editing, as it does on the source. The rows
come in the order of the source, a footnote in its place too, so moving down the preview never
moves up the source.

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
| ```` ```mermaid ```` blocks | the diagram, in the theme's colours, on a tint (below) |
| `-` / `1.` lists, nested | `•` / numbers; a wrapped line goes on under the item's text |
| `- [ ]` / `- [x]` | `☐` / `☑` |
| `> quote`, GitHub alerts (`> [!NOTE]`) | a `│` bar; an alert's title and bar in its colour |
| tables | box drawing, each column aligned as its `:---`, `:---:` or `---:` says |
| `---` | a rule across the pane |
| `[text](url)` | the text, underlined in the link colour; the URL is in the source |
| `![alt](image.png)` | `▣` and the alt text |
| footnotes | the reference as `[1]`; the note where it is written, beside its `[1]` |
| YAML front matter, raw HTML | as written, dim |

Images, heading sizes and math stay text.

A Mermaid diagram is a picture in Ghostty, kitty and WezTerm, which draw pictures with the kitty
graphics protocol; inside tmux, screen or zellij, over mosh, and in a terminal that draws none,
the block stays its source. The picture's text is the size of the terminal's: a diagram narrower
than the pane is drawn as it is, a wider one shrinks to the pane down to half its size, and one
wider still stays its source, as does a diagram the renderer cannot read (#727). A tall diagram
runs over as many rows as it takes. Its rows stand for the block's lines, as a table's rows stand
for its lines: the cursor row runs across it, Enter edits the line it stands for, and `p` shows
the source there. The first diagram of a run is drawn in up to a second, the rest in a tenth; the
source shows until then. An overlay (`?`, a picker) hides the pictures while it is open.

The preview is rendered from the open buffer, not from the disk: an edit shows in it, and a file
an agent rewrites (merl reloads it) renders again. Prose reflows to the width of the pane, emoji
and CJK taking the columns they are drawn in. A table wider than the pane stays in one piece: its
widest columns give up columns first and their cells wrap. A long line of code wraps as it does in
the source, under its indent. The colours come from the theme, so `T` repaints the preview, code
blocks included.

The preview has a cursor row: Up / Down, PgUp / PgDn, Ctrl+D / Ctrl+U and Ctrl+Home / Ctrl+End
move it, and `{` / `}` go to the blank row before the previous or next block. The status bar and
the jump history follow the source line under it. Reading by rows adds no stop, however many
lines a row stands for; Ctrl+Home, Ctrl+End and a far `{` / `}` go by the source's lines and add
one as in the source. A row that shows no line (a table's border, a heading's rule, a blank row
with no blank line under it) stands before the block below: Enter there edits that block's first
line. `[` and `]`, `:`, `o`, `s`, `D`, `t`, `T`, `?` and `q` work as everywhere, and Ctrl+C copies
the source line, nothing on a row that shows none. What acts on a word, a column or a selection
(`/`, `n`, `v`, `d`, `u`, Home / End, Shift or Alt with an arrow, `w`) has nothing to act on in the
preview and does nothing; `p` or Enter take you to the source for it.

In `--review` the diff lives on the source: `p` on a Markdown file of the review renders it as
it stands now (a deleted one as it was) without the diff's marks, and `p` again shows the source
with its diff. A file shown rendered that the branch comes to change stays rendered; a file of the
review opened again shows its source. A file outside the review renders as anywhere; `c` / `C`
there leave the preview and walk the review from the source.
