# What merl is, and what it turned down

The owner's decisions about merl as a whole, each with the issue where it was made. Read this
before you propose a change. An idea under "Turned down" is not proposed again unless the owner
reopens its issue.

## Rules

- merl is a code navigator, not an editor: it reads, follows and reviews code, and types only
  enough to fix a line (#723).
- Less is more: the smallest set of tools the job needs. Nothing to customise beyond the theme
  and a few settings (#540, #723).
- What is left out stays out: no plugins, no language server, no index, no splits or tabs, no
  mouse, no keymap to write (#72).
- `d` never guesses. A wrong jump is worse than a list, and a list is better than a guess (#305).
- Search and the lists read the query as typed and ignore case: no regex, no pattern syntax, no
  match-case key, no toggle (#174, #519).
- A visible change starts from the classic terminal editors (Vim, Neovim, Emacs, Helix): copied
  one to one, or adapted where that is better (#399).
- Review does what GitHub and GitLab both do: lines paired in order, files in the forges' tree
  order, a viewed tick per file (#244, #245, #241).
- The screen carries no help hint: the start screen lists `?`, the status bar never does (#857).
- Review's `A` / `M` / `D` letters are plain: no colour for them held across the themes (#450).
- No theme is removed for its look: a faint selected row is the theme's own (#747).
- An unbound Ctrl, Cmd or Alt+Ctrl chord does nothing (#839).
- Every text, the README included, is plain human language, short, with no filler (#540, #230).

## Turned down

- A review that ties into a forge: queues, comments, publishing, `gh` or `glab` (#552).
- Reshaping the diff: folding moved code, collapsing renames, hiding indentation changes,
  pairing lines by similarity (#552, #244).
- An agent-written brief or summary before a branch, in any form; tests run against the base,
  branch stacks, overlap checks between branches, a review summary handed back to the agent
  (#552).
- Claims that need a model of the project, such as "who depends on this" (#552).
- Moving the view on a jump in any other way: it is centred, with no scroll animation (#552).
- Diff marks inside a review's Markdown preview: prose is read whole (#596, #268).
- A review that opens before the branch has a change (#398), or one started by `c` (#159).
- Opening a file in a merl that is already running, from an agent or a script (#925).
- A key to make the tree wider or narrower (#709).
- Markdown vault features: wiki links, backlinks, front matter as a table (#766); `d` on an
  `@path` import (#1016).
- Bold word starts in prose, Bionic-style (#880).
- Keycaps for the keys a tutor lesson names (#251).
- `.PY` and other upper-case extensions read as their language (#873).
- Search by meaning, until a small local model gets about 9 of 12 on #762's query set (#762).
