# Brief for an implementer in a batch

You build ONE task of github.com/Maksim-Burtsev/merl for a dispatcher. Your task file names the
kind (`epic` or `own PR`), the issue or issues, your worktree, branch and base, a scratch folder,
a release build of master, and the commit trailers. Read `AGENTS.md` in your worktree and follow
`## Working on an issue`, except where this brief differs.

## Everyone

- Work only in your worktree, built into its own `target/`. The main checkout and other
  worktrees belong to others.
- The laptop is shared: run cargo as `CARGO_BUILD_JOBS=2 nice -n 15 cargo …`, tests with
  `RUST_TEST_THREADS=2` in the environment (the sweeps that spawn threads of their own read only
  the variable, never `--test-threads`). Before a release build, a smoke run or a bench, read
  `sysctl -n vm.loadavg` and wait while it is over 16.
- The heavy runs go one at a time on the whole machine: start `tools/d-bench/run`,
  `tools/fold-bench/run`, `tests/smoke/run.py` and the `no_panic` sweep under `lockf -k
  /tmp/merl-heavy.lock …`, which waits for the run another agent holds. Skip the lock only when
  your task file says the owner lifted the limit.
- Commit work in progress; `git stash` is shared by every worktree of the repository, and
  another agent's `pop` takes yours.
- `tools/d-bench/run` gets `--cache` pointing at a private folder of symlinks to
  `~/.cache/merl-d-bench/<project>`: parallel runs overwrite the shared cache's tables.
- "Before" screencasts use the master build from your task file.
- A change to a screen updates the golden screens and snapshots in the same commit.
- Before every push, `tools/ci-local` (the jobs of `.github/workflows/ci.yml`), its output to a
  file; push only when it ends in `all passed`.
- About an hour. Kill hung tmux sessions, builds and probes. Stuck: push what is good and report
  what is left.
- Merging is the dispatcher's.

## Kind `epic`

Push the branch; open no PR and run no review: the dispatcher merges it into the epic. Root
cause, the smallest fix, a test that fails without it (revert the fix and watch it go red). A
fix a user can see gets its `CHANGELOG.md` entry and a before/after pair on `media`.

Report in up to 8 lines: branch, commits, before and after in a line each, the test that catches
it, screencast links, what the change leaves alone.

## Kind `own PR`

Open a PR into master in the shape of `.github/pull_request_template.md`; a visible change gets
before/after screencasts and `needs-owner`. The decisions are in the issue: the owner's latest
comment overrides the body, and what the issue does not decide is not yours to add to the
screen. Review the PR with Punchcard once, its finders in the foreground
(`run_in_background: false`), and fix its findings. A finding that asks whether a change is the
owner's call is not yours to answer: take that change out of the PR and name it in the report.

Report in up to 6 lines: the PR's link, what a user sees now, anything you had to decide.
