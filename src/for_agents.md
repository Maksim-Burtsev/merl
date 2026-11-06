# merl, for an agent

merl is the terminal code navigator your user reads and reviews code in. It knows git and nothing
else: no language server, no model, no forge. It is full-screen, so you never run it yourself
(this guide is the one exception). You prepare things in the repository, and the user opens merl
in a terminal of their own.

What the user does in merl:

- `merl` opens the project: a file tree on the left, the code on the right.
- `merl --review` reviews the checked-out branch against its base; `merl --review=BRANCH` fetches
  BRANCH and checks out what was pushed in a folder of its own (`.git/merl/branches/BRANCH/`),
  leaving the user's checkout as it is. The left panel lists the changed files, the diff is drawn
  over the code, and `c` / `C` walk the hunks file by file, top to bottom of the panel.
- `a` leaves a note for you on a line or a selected range, in a review only (`merl --review`);
  reading a file without a review takes no notes. See "Review notes" below.
- `m` marks a file viewed. Marks are kept per branch across sessions; a file that changed since
  it was marked loses its mark by itself. Never touch them.

The keys are at the end.

## When the user asks you to prepare something in merl

"Prepare these branches for review in merl", "lay out this project for me in merl": you write a
reading order, one small text file, and merl shows the files in that order. Nothing else is
needed: no flag, no command for the user beyond opening merl.

### A review of a branch

For each branch:

1. `git fetch origin BRANCH`, then the files merl will list:
   `git diff --name-status $(git merge-base origin/HEAD origin/BRANCH) origin/BRANCH`.
   merl's base is `origin/HEAD`, or else the first of {BASES} that exists; if the user
   names another base, use that. Read the diff from these
   refs. Do not switch, check out, reset or stash anything: the working tree is the user's, and
   merl checks the branch out itself when they open it.
2. Decide the order (see "The order").
3. Write the order to `$(git rev-parse --git-common-dir)/merl/review/BRANCH`, the branch name
   with its slashes (`.git/merl/review/feat/paging`). Create the directories. The first line is
   `# commit SHA`, the commit of `origin/BRANCH` you read; then the paths, one per line.

When every branch is done, answer with one line per branch, nothing more:

    merl --review=feat/paging
    merl --review=fix/login-rate

### The same branch again

"Look again, they fixed it": the order file is already there. Read its `# commit` line, fetch,
and see what changed since: `git diff --name-status OLD_SHA origin/BRANCH`. Put new files where
they belong, drop the ones the branch no longer changes, move a file whose role changed, and
write the new commit on the first line. Keep the rest as it was: the user has read part of it.
Leave the viewed marks to merl.

### A project to read

Write the order to `$(git rev-parse --git-common-dir)/merl/tree`, the same format, for the files
to read from start to finish. The user opens `merl` in the repository.

## The format

```
# commit 3f2a9c1
# notes are for you; merl skips every line that starts with #
app/model/page.py
app/repo/notes.py
tests/test_paging.py
```

Paths are relative to the repository root, one per line. merl keeps the tree a tree: inside
every directory, its entries go in your order, and a directory stands where its first listed
file is. So you order files within a directory and directories among their neighbours; you
cannot interleave the files of two directories (`src/a`, `tests/a`, `src/b` shows `src/` whole,
then `tests/`). Files you leave out come after the listed ones, in the usual order, so nothing is
ever hidden. A path that is not in the tree or the branch is ignored.

## The order

The user reads the change once, top to bottom of merl's panel, and by the last file should
have the whole picture: what changed and, between the lines, why. Order the files so the
change unfolds in sequence, like a story. A story usually starts where the change shows from
the outside (an endpoint, a command, a screen) and goes inward, toward the data; tests,
wiring and docs come after it, and what is not part of the story goes last, under a `#`
note. Look at as much of the change as you need to decide. Often no order is the single
right one; pick one that reads well. merl keeps a directory's files together, so a directory
is a chapter.

A project to read is the same at the scale of the system: by the last file the user knows
how it works.

### What the user need not read

When the user asks you to leave out what they need not read, write `skip ` before each such
path, in its place in the order (`skip CHANGELOG.md`; `skip docs/` skips every file under it),
and put those paths last. merl keeps the file in its panel with a mark, `c` / `C` walk past it,
and the user can still open it from the panel.

Read a file's diff before you skip it, and skip only what is not production code:

- tests, new or changed;
- docs: the changelog, a page or a comment that describes the change;
- media: pictures, recordings, fonts;
- output a tool generated, and lock files.

Never skip:

- CI, build scripts, dependency manifests, migrations and schemas, binary files other than
  media;
- anything that touches auth, secrets, payments or users' data;
- the part of the change the task asked for.

When in doubt, keep the file: a kept file costs the user a minute, a wrong skip costs a bug
they never saw.

## Review notes

When the user says they reviewed in merl or left notes, read
`$(git rev-parse --git-common-dir)/merl/notes/BRANCH`, BRANCH being the reviewed branch with
its slashes (`.git/merl/notes/feat/paging`). merl rewrites it whole on every change, so it is
current even while merl is open.

```
# merl notes · feat/paging ← main · 3f2a9c1
# The user's review notes, for you. Delete this file when you have dealt with them.

app/paging.py:9-10
>     total = len(items) // limit
>     return [page(items, i * limit, limit) for i in range(total)]
[1, 2, 3] by 2 loses the last page; keep the old range step

app/paging.py:9
-     return [page(items, i, limit) for i in range(0, len(items), limit)]
why drop the range step? it was right
```

A note is a block: `path:LINE` or `path:FIRST-LAST`, the noted lines as they read when the note
was written (`>` a line of the file, `-` a line the branch deleted, which stood right above the
line named), then the note, one or more lines, with one space before its first line when that
line starts with `>` or `-`; a blank line ends the block. Find the place by the quoted lines: line
numbers move.
Each note is an instruction from the user about that place.

- On the user's own branch, or one you wrote: fix every note, answer in the chat note by note
  (done, or why not), then delete the file.
- On someone else's branch (a colleague's MR): change nothing, push nothing; ask the user what
  to do with the notes.
- Never post the notes to GitHub or GitLab, never commit or copy them into the repository.

## Also

- An order applies when merl opens. If the user has merl open already, tell them to reopen it.
- `MERL_ORDER=off merl ...` ignores every order: directories first, then by name.
- Delete an order file to drop it.
- Your harness may ask the user before you write inside `.git/`; that is expected.

## Keys

