# merl — notes for agents

- `src/app/mod.rs` `KEYS` is the single source of truth for the key bindings: the `?` overlay, the
  README `## Keys` table (a test compares them) and the tutorial all derive from it.
- `merl --tutor` (`src/tutor.rs`, sample project in `tutor/notes/`) walks `TUTOR`, a list of
  tasks from `POOL` (`src/tutor/pool.rs`), which `merl --drill` (`src/tutor/drill.rs`) shares:
  one task per action of `KEYS`, each with a start of its own, a tutor text that names the key, a
  drill text that never does, and the answer that does it. When you add, remove or rebind a key, or change what an
  overlay or jump does, update the affected task or add one. The test
  `every_key_is_taught_or_skipped_on_purpose` fails on a new `KEYS` action until a task trains it
  or it is listed in `NOT_TAUGHT`; `every_task_starts_undone_and_its_answer_does_it` fails when a
  task's start already does it or its answer does not, from cold or with the whole pool run
  before it on one App.

## Working on an issue

A brief can be as short as "Work on #N". It is done when the PR is open in the shape below, CI
on it is green, and you report its link as ready to merge.

1. Read the issue with its comments, then check that nobody built it yet: the issue is open and
   `gh pr list --state all --search N` shows no PR for it. Parallel sessions work on this repo.
2. Branch in a worktree of your own, cut from origin: `git fetch origin && git worktree add
   ../merl-<topic> -b <branch> origin/master`. The main checkout is shared: other sessions keep
   their branches checked out there with uncommitted work, and its `master` can be days behind
   origin or carry commits origin never got. Read code, reviews included, from your worktree.
3. Build into the worktree's own `target/`. Worktrees sharing a `CARGO_TARGET_DIR` hand you a
   stale `merl` test binary, one built from another tree; after sharing one, `cargo clean -p
   merl` before trusting a result. A build of master (for a screencast or a sweep) gets a
   worktree and a target of its own too.
4. Keep scratch files (fixtures, GIFs, harnesses) in a folder named after the issue: parallel
   runs share a scratchpad and overwrite each other's `before.gif`.
5. Before a push, run what `.github/workflows/ci.yml` runs. A change a user can see adds its
   entry under `## [Unreleased]` in `CHANGELOG.md`, citing the issue as `(#N)`; docs, refactors,
   tests and tooling get none.
6. Commits are in English and carry the reasoning. The repo squashes with the PR's commit
   messages, so a commit written with Claude keeps its `Co-Authored-By: Claude …` trailer.
7. Record the screencast, open the PR, and check `gh pr diff --name-only` holds only your files.

Everything on GitHub (issues, PR bodies, reviews, comments) is in English.

## Changing `d`

A wrong jump is worse than a picker or "don't know", and no lookup may get worse than on master.
Only the path the change narrows gets new rules: every other lookup, and every other language,
matches exactly as master does. Rules that start answering a new question tend to leak into the
fallback paths, so a `d` PR is ready to merge only after both passes below show no row worse than
master; their tables go into the commit message.

- **Sweep.** Every scenario the tests and the reviews raised, as a TSV of `tag root file line
  col`, played on master and on the branch by a temporary `#[ignore]` test `zz_sweep` in
  `src/app/tests/`, the same file in both worktrees: one `App` per project, set `jump_to` and
  `col`, press `d`, write `shown()` and the milliseconds. `cargo test --release zz_sweep --
  --ignored` plays ~900 cursors in ~40 s. Time in release only: a `grep` call costs ~35 ms in
  debug, ~1 ms in release. Each row is same, better or WORSE. Never commit the harness.
- **Replay** over real projects, `git clone --depth 1` into your scratch folder, cursors picked
  by shape: fastapi, mealie (`uv sync`); gin, gitea; hono (`npm install --ignore-scripts`),
  typeorm, nest, immich (`pnpm install --ignore-scripts --filter 'immich...'`). TypeScript rows
  are checked against TypeScript's own `getDefinitionAtPosition`.

An adversarial fixture per language (shadowed imports, namesake types, declaration-shaped lines
in strings) finds what real projects do not. To prove a test can fail, revert one fix at a time
and watch it go red.

## Pull requests

- The body of a pull request is read by a merl user, not by a reviewer of the diff. It is the
  shape of `.github/pull_request_template.md` and nothing more: the issue it closes, one
  sentence on what happens today, one on what happens now, the before/after screencasts, and a
  `Keys —` line that is always there (`none` when nothing moved). A bullet list only when more
  than one thing is visible; for a single change it would repeat **Now**. Everything else — why
  this design, what was tried, the rules per language — belongs in the commit messages.
- On a PR branch that is not yours, `git fetch` and read `gh pr view N --json
  headRefOid,comments` before you fix anything and again right before you push: the session
  that opened it answers a review within minutes. If the author already fixed it, verify their
  fix instead of pushing a second one; comment on a fix only once your push has landed.

## Screencasts

Anything visible in the interface ships with a before/after screencast; internal refactoring,
docs and tests need none. `tools/cast.py` records one run of one binary as a GIF (its docstring
has the steps grammar); run it twice with the same steps file, once against a build of master,
once against the branch. Record outside the checkout (`cp -R tutor/notes /tmp/notes`) and keep a
run under ~15 s.

- Open the steps with `wait <text merl draws>` and a no-op `key Escape`: leading waits are not
  sampled, so the first real key would otherwise change the screen before the GIF starts.
- Move the way a person does: `Alt+Left` / `Alt+Right` to a word, `End`, `/WORD`, `c` / `C` to a
  hunk, `[` back. A cursor crawling with `Right*28` argues that merl is slow.
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

## Merging

`master` takes squash merges of PRs only, with the CI checks green on a branch up to date with
master; nobody can push to it directly or bypass the checks.

- After another PR lands, `gh pr update-branch N` and wait for the checks again.
- A PR that conflicts with master gets no CI at all on a push. Merge `origin/master` into the
  branch, resolve, push: a merge keeps the commits a posted review links to, and the squash keeps
  it off master.
- In a stack, never `gh pr merge --delete-branch` a PR another one is based on: GitHub closes the
  upper PR for good. Merge the base, rebase the next branch onto `origin/master`, force-push,
  `gh pr edit --base master`. CI runs only for PRs into master, so push a new commit if it does
  not start after the retarget.

## Releases

Only when the owner asks for one. In order:

1. The changelog covers every PR since the last tag (`git log vX.Y.Z..origin/master`): check
   each commit's issue and content, since PRs merge without an entry, and parallel merges leave a
   second `### Added` in `## [Unreleased]` to fold into the first. Dependabot, `docs:`,
   `refactor:` and tooling commits get no entry. Entries cite the issue.
2. A release PR, `release: X.Y.Z`: `## [Unreleased]` becomes `## [X.Y.Z] - YYYY-MM-DD` with its
   link, and the version goes into `Cargo.toml` and `Cargo.lock`.
3. After the merge, an annotated tag `vX.Y.Z` (message `merl X.Y.Z`) on that commit, pushed;
   `release.yml` builds the GitHub release and its binaries.
4. The Homebrew tap: `release.yml` bumps it when the `TAP_TOKEN` secret is set; otherwise bump
   `Formula/merl.rb` in `Maksim-Burtsev/homebrew-tap` by hand (the version and the three sha256
   of the `.sha256` assets), commit `merl X.Y.Z` and push.
