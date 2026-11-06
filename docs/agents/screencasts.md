# Screencasts

`tools/cast.py` records one run of one binary as a GIF (its docstring
has the steps grammar); run it twice with the same steps file, once against a build of master,
once against the branch. Record outside the checkout (`cp -R tutor/notes /tmp/notes`) and keep a
run under ~15 s.

- Open the steps with `wait <text merl draws>` and a no-op `key Escape`: leading waits are not
  sampled, so the first real key would otherwise change the screen before the GIF starts.
- A screencast that moves through code moves the way a seasoned keyboard user does, someone with
  twenty years of Vim or Emacs: the keys that person reaches for, at that person's pace. The owner
  scrolled files with a mouse for years and learns keyboard navigation from these screencasts.
  - Reading down a file is `Ctrl+D` / `Ctrl+U`, the keyboard's scroll wheel. Code already on the
    screen is reached the way a person would reach it there: a few `}`, or `/` and the first
    letters of a name. Either is fine; the walk takes whichever comes naturally in that spot,
    not whichever has the fewest keys. Code off the screen is reached by name (`D`, `d`, `s`), by
    number (`:N`), or back (`[`). Arrows move a line or two.
  - The keys play at `tools/cast.py`'s pace, each shown in its keycap, and the walk pauses where
    a person would read.
  - A demo of something that is not a walk (a picker, an overlay, a flag) shows it directly.
- When merl exits (a panic, `q`) the pane dies and the GIF stops a frame early: pass `--bin` a
  script that runs merl and then `sleep 6`. For a change to the command line, the script is a
  prompt that runs what the steps type, the binary passed after `--`:
  `BIN=$1; merl() { "$BIN" "$@"; }; while printf '$ ' && read -r line; do eval "$line"; done`.
- `media` is public: no real home directory and no scratch path in a frame. Build a fake home in
  `/tmp`, and pipe a command's output through `sed "s|$PWD/||"`.
- Upload the pair to the orphan `media` branch, `prs/<N>-<slug>-before.gif` and `-after.gif`
  (a bug GIF for an issue goes under `issues/`): `gh api -X PUT
  repos/Maksim-Burtsev/merl/contents/prs/<name> --input -` with `{message, branch: "media",
  content: <base64>}`. Every upload gets a name that is not taken yet (`GET …?ref=media`):
  GitHub's image proxy caches by URL, so an overwritten GIF keeps showing the old one. Nothing is
  deleted from `media`; merged PR bodies link to it.
- The README's GIFs are made by `assets/tapes/record.py`, not cast.py; see its docstring.
