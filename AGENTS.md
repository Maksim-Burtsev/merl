# merl — notes for agents

- `src/app/mod.rs` `KEYS` is the single source of truth for the key bindings: the `?` overlay, the
  README `## Keys` table (a test compares them) and the tutorial all derive from it.
- `merl --tutor` (`src/tutor.rs`, sample project in `tutor/notes/`) walks `TUTOR`, a list of
  tasks from `POOL` (`src/tutor/pool.rs`), which `merl --drill` (`src/tutor/drill.rs`) shares:
  one task per action of `KEYS`, each with a start of its own, a tutor text that names the key, a
  drill text that never does, and the answer that does it. When you add, remove or rebind a key,
  or change what an overlay or jump does, update the affected task or add one. The test
  `every_key_is_taught_or_skipped_on_purpose` fails on a new `KEYS` action until a task trains it
  or it is listed in `NOT_TAUGHT`; `every_task_starts_undone_and_its_answer_does_it` fails when a
  task's start already does it or its answer does not, from cold or with the whole pool run
  before it on one App.
- `tests/smoke/*.steps` are the scenarios the release smoke test plays in tmux on the build and on
  the last release (`.claude/skills/smoke-test/SKILL.md`). A PR that adds or changes a feature a
  user can see (an `### Added` or `### Changed` entry) plays it in a scenario in the same PR, its
  comment citing `(#N)`: the release then reads the entry `seen working` instead of `not checked`,
  and every later release guards it. Append to the scenario closest to the feature, or add a file;
  the last release stops at the first wait for what it lacks, so the new steps go at the end.
  Keep the whole run under ~3 min (`wall` in the report's header) and check the scenario with
  `tests/smoke/run.py --only NAME`: PASS or DIFF on the new build.
- CI plays the scenarios on every PR (`tests/smoke/run.py --golden`, all but `RELEASE_ONLY` in
  `run.py`) and fails on any checkpoint whose screen, as text, differs from its file under
  `tests/smoke/screens/SCENARIO/`. A visible change, or new steps in a scenario, updates those
  files in the same PR: `tests/smoke/run.py --update --only NAME`, then commit them, so the PR's
  diff shows every screen it changes. Check each rewritten screen shows what the change meant.
- A wait is the checkpoint's sync, so it names the last thing the screen draws. The tree's git
  marks come from a thread after merl starts: a checkpoint right after a start, or after a `run`
  that adds a file the tree shows, waits for that file's mark (`wait A attack.rb`), not for the
  code drawn at once (#1036).
- Beside the screens, `keys.txt` lists the `KEYS` actions merl counted a press of in the play.
  `every_key_and_flag_is_smoked_or_skipped_on_purpose` (`src/app/tests/smoke.rs`) fails on an
  action of `KEYS` or a flag of the command line that no scenario plays: a new key or flag gets
  steps in a scenario and its `keys.txt` rewritten by `--update`, or a line in `NOT_SMOKED` with
  why.
- A PR that changes a text a scenario waits for updates that scenario in the same PR; `grep -rn
  'TEXT' tests/smoke assets/*.steps` finds them all, the README's tapes that `scale` plays
  included. The wait moves to text both the change and the last release draw: the change then
  shows as a difference in its checkpoint, while a wait on the new text would stop the last
  release there and leave the rest of the scenario unplayed.
- `src/app/tests/edit_fuzz.rs` plays random edit sessions on `App` and on a plain model of the
  edit keys, comparing text, cursor, selection, clipboard and the file on disk after every key.
  A change to what an edit key does changes the model in the same PR; before pushing a change to
  `src/app/edit.rs`, run `MERL_FUZZ_SESSIONS=5000 cargo test --release edit_fuzz` (a failure
  prints its seed and the shortest key sequence that still fails).
- `tests/snapshots/*.txt` are whole screens, text and colours, of every overlay, picker and panel
  at 80×24, 120×33 and 185×55 in a dark and a light theme (`src/ui/tests/snapshots.rs`). A PR
  that changes what a screen draws updates them in the same PR: `MERL_UPDATE_SNAPSHOTS=1 cargo
  test snapshots`, then read `git diff tests/snapshots` as the change seen on screen. A new
  screen, overlay or panel adds its state to `STATES`.
- No file under `src/` passes 1,500 lines of non-test code, and the ones already over it may not
  grow: `no_source_file_grows_past_its_size` (`src/main.rs`, `LONG_FILES`). Split the file; a
  listed file that shrinks lowers its number in the same PR.
- Code carries no comments, `///` and `//!` included: names, types and tests say what it does,
  and the commit message says why, a `ponytail:` note included. Fixture annotations
  (`tests/fixtures/`) and the tutor's sample project are data and stay. `src_holds_no_comment`
  (`src/main.rs`) fails on any comment under `src/` and names its file and line;
  `scripts_and_config_hold_no_comment_but_the_kept_ones` does the same for the Python, shell,
  Lua, CI and Cargo files, Python docstrings checked by hand. A comment stays only in
  `KEPT_COMMENTS` beside them, with why the code cannot do without it: a tool reads it (a
  docstring printed as the program's usage), or a licence requires it.
- The lints live in `Cargo.toml` `[lints]` and `clippy.toml`, and `tools/ci-local clippy` denies
  every warning. `#[allow]` is banned: a lint is silenced only by `#[expect(lint, reason = "…")]`,
  and `expects_do_not_grow` (`src/main.rs`) keeps their number under `EXPECTS`; a PR that removes
  some lowers it. `unwrap()` and `&s[a..b]` on a `str` are not used outside tests.
- The site (#776) runs this same `src/` in the browser: `web/build.sh OUT.wasm` builds it for
  `wasm32-wasip1` from `web/Cargo.toml`, whose dependencies match `Cargo.toml`'s
  (`the_browser_build_has_merl_s_dependencies`), at the versions `Cargo.lock` holds. The browser
  has no terminal, threads, processes or `canonicalize`: code that needs one stays out of it by
  `#[cfg(not(target_family = "wasm"))]` or goes through `src/real.rs`, and git answers from a replay `web/review/bundle.py` records
  from a native `--review`. A change to what `--review` asks git re-records it; `site.yml`, on
  every PR that touches `src/` or `web/`, builds it and plays the quest, failing on a step that
  no longer passes or a git call the replay lacks.

## Issues

The `1.0` milestone holds what 1.0 needs, and its description gives the order. Every open issue
carries three labels:

- What it is: `bug` or `enhancement`, plus `tests`, `language-support` or `readme` when it is one
  of those.
- How ready it is: `agent-ok`, `needs-owner`, `to-think` or `backlog`
  (`docs/agents/merging.md` says what each means).
- How soon: `P1` is the 1.0 checklist and goes first, `P2` is wanted for 1.0, `P3` is for when
  time allows.

A large piece of work is an `epic`: a parent issue whose sub-issues are the work. The next issue
to take is the top of `is:open label:agent-ok label:P1 -label:epic -label:in-progress`. An
issue you file gets its three labels, the milestone if 1.0 needs it (never a `to-think` one), and
its epic as the parent when one fits (`gh issue edit EPIC --add-sub-issue N`).

An issue's body opens with `Visible:`: `no`, or what the user will see change. A visible issue
gets `agent-ok` only with a `## Proposal` in its body that the owner has picked from
(`.claude/skills/proposal/SKILL.md` shapes one): what is built before the owner saw it tends to
be built twice. A visible change the owner takes as an experiment ships behind a hidden setting,
off by default.

Several `agent-ok` issues at once, an evening queue or a night run, go as a batch:
`.claude/skills/batch/SKILL.md`.

## Explaining to the owner

The owner judges merl by what they press and what they see. An explanation of a bug, a fix or
an idea is that walk-through in a few lines: the keys, the screen before, the screen after, and
whether anything visible changes. Code, file paths and research come only when asked for. A
choice comes as options with a recommendation, and a look comes as a picture
(`.claude/skills/proposal/SKILL.md`). Sorting issues with the owner:
`.claude/skills/groom/SKILL.md`.

## Working on an issue

A brief can be as short as "Work on #N". It is done when the PR, in the shape below, is merged
with its issue closed, or waits under `needs-owner` (see `docs/agents/merging.md`).

1. Read the issue with its comments, then check that nobody has it: the issue is open, carries
   no `in-progress` label, and `gh pr list --state all --search N` shows no PR for it. Parallel
   sessions work on this repo. Then take it, before anything else: `gh issue edit N --add-label
   in-progress` and a comment naming your branch. If you stop without a PR, take the label off
   and say in a comment what is left.
2. Branch in a worktree of your own, cut from origin: `tools/worktree ../merl-<topic> <branch>`,
   which also starts its `target/` with master's dependencies built. The main checkout is shared: other sessions keep
   their branches checked out there with uncommitted work, and its `master` can be days behind
   origin or carry commits origin never got. Read code, reviews included, from your worktree.
3. Build into the worktree's own `target/`. Worktrees sharing a `CARGO_TARGET_DIR` hand you a
   stale `merl` test binary, one built from another tree; after sharing one, `cargo clean -p
   merl` before trusting a result. A build of master (for a screencast) gets a
   worktree and a target of its own too.
4. Keep scratch files (fixtures, GIFs, harnesses) in a folder named after the issue: parallel
   runs share a scratchpad and overwrite each other's `before.gif`.
5. Before a push, `tools/ci-local`: every job of `.github/workflows/ci.yml`, which calls it step
   by step (`tools/ci-local clippy tests` runs only those), and the Deny workflow's `cargo deny`
   (`deny`), stopping non-zero at the first that fails. A change a user can see adds its
   changelog entry as a file of its own, `changes/N.md`, `N` the issue (two issues, two files),
   never into `CHANGELOG.md`, whose `## [Unreleased]` stays empty until a release gathers the
   files: so parallel PRs never conflict on it. The file is the heading and the entry as the
   changelog will read them, citing the issue as `(#N)`:

   ```
   ### Fixed

   - `d` no longer … (#N)
   ```

   The heading is one of Keep a Changelog's (Added, Changed, Deprecated, Removed, Fixed,
   Security); `every_change_is_an_issue_s_entry_under_a_keep_a_changelog_heading` checks the
   name, the heading and the `(#N)`. Docs, refactors, tests and tooling get none. What the change
   moves in the tests, the bullets at the top saying how:

   | The PR changes | It moves |
   |---|---|
   | a key or a flag | a tutor/drill task, smoke steps and their `keys.txt` |
   | what a screen draws | `tests/snapshots`, and the smoke screens it shows in |
   | a new screen, overlay or panel | its state in `STATES` |
   | what an edit key does | the `edit_fuzz` model |
   | where `d` lands | a fixture annotation, and the bench's baseline |
   | an `### Added` or `### Changed` entry in `changes/N.md` | smoke steps citing `(#N)`, or a fixture annotation |
   | a bug | a test that fails without the fix |

   The `### Added` / `### Changed` row is enforced:
   `every_unreleased_feature_is_smoked_or_skipped_on_purpose` (`src/app/tests/smoke.rs`) fails on
   an entry, in `changes/` or under `## [Unreleased]`, no scenario or fixture cites, until its
   issue is listed in `UNSMOKED` with why.
6. Commits are in English and carry the reasoning. The repo squashes with the PR's commit
   messages, so a commit written with Claude keeps its `Co-Authored-By: Claude …` trailer.
7. Record the screencast (`docs/agents/screencasts.md`), open the PR, and check `gh pr diff
   --name-only` holds only your files. The PR says `Closes #N` only when the issue's own example
   now does what the issue wants: run it on the build. Otherwise it says `Part of #N`, and what
   is left is filed as an issue.
8. Once the whole task is in the PR, review it with Punchcard, once. Fix every finding on the
   branch's own change in this PR; a finding outside it becomes an issue. Do not review again:
   with the fixes pushed, go on as `docs/agents/merging.md` says. A docs-only PR needs no review.

Everything on GitHub (issues, PR bodies, reviews, comments) is in English.

## Features that read code

`d` and `f` are two of the features that infer the structure of code; the rule below holds for
every one of them, new ones included. A wrong answer is worse than "don't know", and that is
proven on a bench of real projects against a reference, not argued from a few examples. A
comparison with another editor ("VS Code folds this") cites the source that was opened: a file
and line of its code, or a run of it, never a recollection.

## Changing `f`

Before you change what `f` folds or turn it on in a language (`src/app/collapse*`, `tests/folds/`,
`tools/fold-bench/`), read `docs/agents/f.md`.

## Changing syntax colours

Before you change how merl colours code (a grammar, `src/treesitter*`, `queries/`, `themes/`,
`tests/colors/`, `tools/theme-bench/`) or `Buffer::edited`, read `docs/agents/colours.md`.

## Changing `d`

Before you change where `d` jumps in any language, its fixtures (`tests/fixtures/`) or its bench
(`tools/d-bench/`), read `docs/agents/d.md`.

## Pull requests

- One issue, one PR. An issue whose options the owner has not picked from is not built; when
  the owner asks for a PR anyway, it holds the recommended option, and the others are
  screencasts in its body (`.claude/skills/proposal/SKILL.md`), never PRs of their own.
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
docs and tests need none. Before you record a screencast or GIF, or upload anything to `media`,
read `docs/agents/screencasts.md`.

## Merging

Before you merge a PR, leave it for the owner, act on the owner's comment on it, or update one
that conflicts with master or sits in a stack, and before you set `needs-owner`, `to-think` or
`release-blocker`, read `docs/agents/merging.md`.

## Releases

When the owner asks for a release, and only then, follow `docs/agents/releases.md` in order.
