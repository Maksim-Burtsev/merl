# A day with merl

One review, start to finish. The recordings are made by `assets/tapes/record.py` from the `.steps`
file next to each GIF.

**1. The agent says the branch is done.** `merl --review` opens it on its first hunk. The panel
lists what the branch touched. `merl --review feature-x` fetches it and the base first and
checks out what was pushed, `--base origin/dev` compares against another base.

<a href="../assets/day/1-review.gif"><img src="../assets/day/1-review-page.gif" alt="merl --review opens the agent's branch on its first hunk" width="800"></a>

**2. `c` walks the hunks**, through the file and on into the next one. `C` walks back. Images
and other binaries are skipped, and merl says how many. A file `c` has walked to the end gets a
tick in the panel; `m` puts it or takes it off by hand, and it goes on its own when the agent
changes the file, as on GitLab. The ticks stay with the branch: the next `merl --review` of it
opens with them.

<a href="../assets/day/2-hunks.gif"><img src="../assets/day/2-hunks-page.gif" alt="c walks from hunk to hunk and into the next file" width="800"></a>

**3. A hunk calls something you do not know.** `Alt+Left` / `Alt+Right` hop word by word onto it,
`d` opens its definition, `u` lists who else calls it.

<a href="../assets/day/3-into.gif"><img src="../assets/day/3-into-page.gif" alt="d from a changed line to the definition the branch added, then u for its usages" width="800"></a>

**4. `[` goes back** to the hunk you left, however far you wandered: one `[` per jump.

<a href="../assets/day/4-back.gif"><img src="../assets/day/4-back-page.gif" alt="two [ walk back from the engine through the helper to the hunk" width="800"></a>

**5. The agent is still working** in the other split. The file it just wrote shows up in the
review on its own, and `c` will walk into it.

<a href="../assets/day/5-live.gif"><img src="../assets/day/5-live-page.gif" alt="a file the agent writes appears in the review panel" width="800"></a>

Nothing here types into the branch: a review is reading, and a fix typed over it reaches no pull
request. When the line is yours to change, Enter and Esc are in [Editing](editing.md).

