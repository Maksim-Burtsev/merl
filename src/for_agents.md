# merl, for an agent

merl is the terminal code editor your user reads and reviews code in. It knows git and nothing
else: no language server, no model, no forge. It is full-screen, so you never run it yourself
(this guide is the one exception). You prepare things in the repository, and the user opens merl
in a terminal of their own.

What the user does in merl:

- `merl` opens the project: a file tree on the left, the code on the right.
- `merl --review` reviews the checked-out branch against its base; `merl --review=BRANCH` fetches
  BRANCH and checks out what was pushed. The left panel lists the changed files, the diff is drawn
  over the code, and `c` / `C` walk the hunks file by file, top to bottom of the panel.
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
   merl's base is `origin/HEAD`, or else the first of `origin/master`, `origin/main`,
   `origin/develop` that exists; if the user names another base, use that. Read the diff from these
   refs. Do not switch, check out, reset or stash anything: the working tree is the user's, and
   merl checks the branch out itself when they open it.
2. Read the change and decide the order to read it in (see "A good order").
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

## A good order

- What is used comes before what uses it: a type or a function before its callers.
- The heart of the change first; mechanical changes (a parameter passed through, a rename),
  docs, the changelog and generated files last.
- Tests right after the code they test, as far as the directories allow.
- In a project to read: what it is (README), its entry point, the core, the data it passes
  around, then the edges (I/O, CLI, alternatives), then support code.

## Also

- merl reads the order when it starts, and an open review again whenever the branch changes.
- `MERL_ORDER=off merl ...` ignores every order: directories first, then by name.
- Delete an order file to drop it.
- Your harness may ask the user before you write inside `.git/`; that is expected.

## Keys

