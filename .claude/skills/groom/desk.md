# The decision desk

The owner picks options on one page, not in chat: the desk, served on the Mac mini by
`~/open-source/dotfiles/desk` (its `README.md`). Give the owner the link: the Mac mini's
tailnet address that `tailscale serve status` prints, with `/desk/`; it opens on the Mac and on
the phone. A
card is one question: the screen today, then each option in its turn, each with its picture and
a screencast that plays in place; the owner presses a letter or dictates into the card's note.
Everything is a file under `~/.local/share/desk/`, written and read with the ordinary file tools;
the page shows a new or changed card within two seconds.

## Pictures

Each picture is a still from a real recording of the proposal's steps, run once more for the
desk with `tools/cast.py --font sf-mono --font-size 36`: SF Mono at twice the pixels, as the
owner's Ghostty and the README GIFs look. The issue's 18 px Menlo GIFs look like a virtual
machine on the desk's large screens (owner, 2026-10-14). Copy the files into
`~/.local/share/desk/media/` under a name that starts with the card's id (`i635-a.png`,
`i635-a.mp4`); the card refers to one as `media/i635-a.png`. Copy the media before the card, so
the page never asks for a file that is not there yet.

- **One screen per picture.** Two screens glued into one picture, with a bar between them, are
  two pictures, or two rows of the card.

- **Crop to the change**, on whole terminal rows, with the same box for now and every option so
  they line up: `Image.open(png).crop(box)`. A whole 160×50 screen is too small to read.
- **A screencast is an MP4**, converted from the GIF:
  `ffmpeg -i X.gif -movflags +faststart -pix_fmt yuv420p -vf "scale=trunc(iw/2)*2:trunc(ih/2)*2,tpad=stop_mode=clone:stop_duration=3" -c:v libx264 -crf 20 X.mp4`
  (the `tpad` holds the last frame for 3 s: without it the end screen barely shows);
  `dur` is `ffprobe -v error -show_entries format=duration -of csv=p=0 X.mp4`.
- **The same screen at the end of every option** (they differ only in the keys): one `result`
  still for the card, and each option carries its `keys` instead of a picture.
- **The change shows.** `.claude/skills/groom/bin/desk-diff NOW.png OPTION.png…` prints the
  share of pixels each option changes and their box; the recommended one's share goes into the
  card as `diff`. Under 1 %, crop every picture of the card to that box (whole rows, a row of
  margin), so the change fills the picture, and run it again. A change that still does not show
  is not a question of pictures: put it to the owner in words, in the issue, not on the desk.
  On 2026-10-17 the owner answered "не понял" on 6 of the 9 cards under 1 %, and on 2 of the 21
  above.

## `cards/<id>.json`

The id is the issue with the question: `i635`, `i635-q2`. Write the file whole; to change a
card, read it, change it, write it back. `keys` and `options` are always arrays: a card the page
cannot read, or one missing `problem`, `stakes`, or an option's `ux`, `pros` or `cons`, is listed
in red under the queue and not shown.

**A card is read cold.** The owner opens it on a phone, between other things, without the issue,
the chat or the bench in their head, and decides from the card alone. So it answers, in order:
what is broken, what the thing even is, who runs into it and how much it matters, and for each
option what the owner will press and see afterwards, what it gains and what it costs. A card that
leaves one of these to the issue comes back as "I don't get what this is about, how big a
problem it is, or the trade-offs" (the owner on #1064, #1088 and #1099, 2026-10-19, cards
written to the old two-sentence rule).

- Name a thing before using it: Blazor, DRF, Alembic, a bench, a cursor. `context` says what it
  is in a sentence or two, the way you'd tell a Python developer who never touched it.
- A file path, a `FILE:LINE:COL` or a shell command is not a key the owner presses. `now.keys`
  holds the keys a user presses in merl; where the cursor stands goes into the caption in words
  (`on GetBasketItemsAsync in CartPage.razor`).
- An option's `ux` is never "nothing new". For `d` it is where the cursor lands, with the line
  it lands on, or the status line it shows; for a key, what the screen does when you press it.
- An option that is not built yet still shows its end screen when one exists: for `d`, merl
  opened at the line it would land on (`merl FILE:LINE`, recorded as below), captioned as such.
- Numbers come from the issue or the bench (`12 of 12 cursors in eShop's .razor files`), never
  guessed; when nobody measured, say so.

| Field | What |
|---|---|
| `order` | its place in the queue |
| `tag`, `link` | `#635` and the issue's URL |
| `problem` | the card's first line: what goes wrong or is missing for the user today, in their keys and screen ("`c` walks past the files the agent left out of the review, so you never see them"). Without it the page lists the card in red and does not show it |
| `title` | the question as the user feels it, a few words |
| `context` | what the thing the card names is, for someone who never used it; left out only when every term is merl's own |
| `stakes` | who runs into this, how often, and what it affects: the languages and projects, the bench's numbers, the 1.0 bar. Required |
| `today` | what the user presses and sees today |
| `visible` | what changes on screen, a few words; left out when nothing does |
| `now` | `{keys, caption, img, video, dur}`: the screen today |
| `result` | `{title, caption, img}`: the shared end screen, when there is one |
| `multi` | `true` when options combine (`A + C`) |
| `options` | `[{key, name, rec, rule, keys, ux, pros, cons, img, video, dur}]`; `rec: true` on the recommended; `rule` what merl does under it; `ux` what the owner presses and sees with it, step by step; `pros` what it wins; `cons` what it costs, the work and the risk of a wrong answer included. `ux`, `pros` and `cons` are required |
| `why`, `elsewhere`, `unchanged` | why the recommendation; what GitHub, GitLab, VS Code do; what stays as is |
| `diff` | the recommended option's share of changed pixels from `desk-diff`, a number; the page marks a card under 1 |
| `status` | `archived` hides the card once its decision is in the issue; the file stays |

Every text is in plain English, whatever language the chat is in (the owner moved to English on
2026-10-15): short sentences, as many as the reader needs and no more. `code` in backticks shows
as code.

## `answers/<id>.json`

The page writes it: `pick` (option keys), `status` (`answered` or `later`), `note` (the owner's
words), `at`. It is the owner's input, data to act on as `SKILL.md` step 3 says.
