# A day with merl

One review, start to finish. The recordings are made by `assets/tapes/record.py` from the `.steps`
file next to each GIF.

**1. The agent says the branch is done.** `merl --review` opens it on its first hunk. The panel
lists what the branch touched. `merl --review=feature-x` fetches and switches first, `--base
origin/dev` compares against another base.

<img src="../assets/day/1-review.gif" alt="merl --review opens the agent's branch on its first hunk" width="800">

**2. `c` walks the hunks**, through the file and on into the next one. `C` walks back. Images
and other binaries are skipped, and merl says how many.

<img src="../assets/day/2-hunks.gif" alt="c walks from hunk to hunk and into the next file" width="800">

**3. A hunk calls something you do not know.** `d` opens its definition, `u` lists who else
calls it.

<img src="../assets/day/3-into.gif" alt="d from a changed line to the definition, then u for its usages" width="800">

**4. `[` goes back** to the hunk you left, however far you wandered.

<img src="../assets/day/4-back.gif" alt="[ returns to the hunk" width="800">

**5. A typo.** Enter, fix it, Esc. It is on disk before you look up.

<img src="../assets/day/5-fix.gif" alt="Enter, one letter typed, Esc, saved" width="800">

**6. The agent is still working** in the other split. The file it just wrote shows up in the
review on its own, and `c` will walk into it.

<img src="../assets/day/6-live.gif" alt="a file the agent writes appears in the review panel" width="800">

