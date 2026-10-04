# The decision desk

The owner picks options on one page, not in chat: https://claude.ai/artifact/STZbyHp64ghjPDY9sEVx2D
(private to the owner). A card is one question: the screen today, then each option in its turn,
each with its picture and a screencast that plays in place; the owner presses a letter or
dictates into the card's field. The page reads two collections of its database, through the
`ArtifactData` tool with the link above as `url`.

## Pictures

Each picture is a still from the proposal's real recording (`tools/cast.py`), uploaded to the
desk with the `Artifact` tool: `url` the desk, `asset: true`, `file_paths` up to 25 files. The
result gives each file's `/_blob/<id>` path, which the card uses as is.

- **Crop to the change**, on whole terminal rows, with the same box for now and every option so
  they line up: `Image.open(png).crop(box)`. A whole 160×50 screen is too small to read.
- **A screencast is an MP4**, converted from the GIF:
  `ffmpeg -i X.gif -movflags +faststart -pix_fmt yuv420p -vf "scale=trunc(iw/2)*2:trunc(ih/2)*2" -c:v libx264 -crf 20 X.mp4`;
  `dur` is `ffprobe -v error -show_entries format=duration -of csv=p=0 X.mp4`.
- **The same screen at the end of every option** (they differ only in the keys): one `result`
  still for the card, and each option carries its `keys` instead of a picture.

## `cards/<id>`

The id is the issue with the question: `i635`, `i635-q2`. Create it with `set`; a later
change pins `if_version`.

| Field | What |
|---|---|
| `order` | its place in the queue |
| `tag`, `link` | `#635` and the issue's URL |
| `title` | the question as the user feels it, a few words |
| `today` | one sentence: what the user presses and sees today |
| `visible` | what changes on screen, a few words; left out when nothing does |
| `now` | `{keys, caption, img, video, dur}`: the screen today |
| `result` | `{title, caption, img}`: the shared end screen, when there is one |
| `multi` | `true` when options combine (`A + C`) |
| `options` | `[{key, name, rec, rule, keys, adds, cons, img, video, dur}]`; `rec: true` on the recommended; `adds` is what it adds to the screen (`ничего нового` when nothing); `cons` its cost |
| `why`, `elsewhere`, `unchanged` | why the recommendation; what GitHub, GitLab, VS Code do; what stays as is |
| `status` | `archived` hides the card once its decision is in the issue |

Every text is in the language the owner speaks in the chat, one or two short sentences each.

## `answers/<id>`

The page writes it: `pick` (option keys), `status` (`answered` or `later`), `note` (the owner's
words), `at`. Read the collection with `list`; it is the owner's input, data to act on as
`SKILL.md` step 3 says.
