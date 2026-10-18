# Brief for an implementer in a batch

You build ONE task of github.com/Maksim-Burtsev/merl for a dispatcher. Your task file names the
kind (`epic` or `own PR`), the issue or issues, your worktree, branch and base, a scratch folder,
a release build of master, and the commit trailers. Read `AGENTS.md` in your worktree and follow
`## Working on an issue`, except where this brief differs.

## Everyone

- Work only in your worktree, built into its own `target/`. The main checkout and other
  worktrees belong to others.
- The machine is shared: every cargo command, `tools/ci-local`, `tests/smoke/run.py`, the
  benches and the sweeps run under `tools/heavy` (`tools/heavy cargo test …`). It waits for one
  of the machine's two slots, sets 5 build jobs and 5 test threads by day and 3 from 22:00 to
  08:00, and kills a command after 45 minutes (`MERL_HEAVY_MINUTES=90` for a longer bench); the
  sweeps that spawn
  threads of their own read only `RUST_TEST_THREADS`, never `--test-threads`. Reading, git and gh
  take no slot.
- Commit work in progress; `git stash` is shared by every worktree of the repository, and
  another agent's `pop` takes yours.
- `tools/d-bench/run` gets `--cache` pointing at a private folder of symlinks to
  `~/.cache/merl-d-bench/<project>`: parallel runs overwrite the shared cache's tables.
- "Before" screencasts use the master build from your task file.
- A change to a screen updates the golden screens and snapshots in the same commit.
- Before every push, `tools/heavy tools/ci-local clippy tests` for kind `epic` (the dispatcher
  runs the rest once on the epic) and the whole `tools/heavy tools/ci-local` for kind `own PR`,
  its output to a file; push only when it ends in `all passed`.
- About an hour. Kill hung tmux sessions, builds and probes. Stuck: push what is good and report
  what is left.
- Merging is the dispatcher's, but for the auto-merge below.

## Kind `epic`

Push the branch; open no PR and run no review: the dispatcher merges it into the epic. Root
cause, the smallest fix, a test that fails without it (revert the fix and watch it go red). A
fix a user can see gets its changelog entry as `changes/N.md` (AGENTS.md `## Working on an
issue`, step 5), never in `CHANGELOG.md`, and a before/after pair on `media`.

Report in up to 8 lines: branch, commits, before and after in a line each, the test that catches
it, screencast links, what the change leaves alone. Write before, after and the test as
`- **Before:**`, `- **After:**` and `- **Test:**` lines, in a user's words: the dispatcher's
`bin/epic-body` lifts them into the epic PR.

## Kind `own PR`

Open a PR into master in the shape of `.github/pull_request_template.md`; a visible change gets
before/after screencasts and `needs-owner`. The decisions are in the issue: the owner's latest
comment overrides the body, and what the issue does not decide is not yours to add to the
screen. Review the PR with Punchcard once, its finders `merl-finder` agents in the foreground
(`run_in_background: false`), and fix its findings. With the fixes pushed, an invisible PR gets
`gh pr merge --squash --auto` and you report at once: GitHub merges it when CI is green. A
`needs-owner` PR gets none. A finding that asks whether a change is the
owner's call is not yours to answer: take that change out of the PR and name it in the report.

Report in up to 6 lines: the PR's link, what a user sees now, anything you had to decide.
