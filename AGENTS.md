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
  and the commit message says why. The comments already in the code are older practice, not the
  density to match: when you change the code under one, delete it and move what still holds into
  the commit message, a `ponytail:` note included. Fixture annotations (`tests/fixtures/`) and
  the tutor's sample project are data and stay.

## Issues

The `1.0` milestone holds what 1.0 needs, and its description gives the order. Every open issue
carries three labels:

- What it is: `bug` or `enhancement`, plus `tests`, `language-support` or `readme` when it is one
  of those.
- How ready it is: `agent-ok`, `needs-owner`, `to-think` or `backlog` (`## Merging` says what each
  means).
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
with its issue closed, or waits under `needs-owner` (see `## Merging`).

1. Read the issue with its comments, then check that nobody has it: the issue is open, carries
   no `in-progress` label, and `gh pr list --state all --search N` shows no PR for it. Parallel
   sessions work on this repo. Then take it, before anything else: `gh issue edit N --add-label
   in-progress` and a comment naming your branch. If you stop without a PR, take the label off
   and say in a comment what is left.
2. Branch in a worktree of your own, cut from origin: `git fetch origin && git worktree add
   ../merl-<topic> -b <branch> origin/master`. The main checkout is shared: other sessions keep
   their branches checked out there with uncommitted work, and its `master` can be days behind
   origin or carry commits origin never got. Read code, reviews included, from your worktree.
3. Build into the worktree's own `target/`. Worktrees sharing a `CARGO_TARGET_DIR` hand you a
   stale `merl` test binary, one built from another tree; after sharing one, `cargo clean -p
   merl` before trusting a result. A build of master (for a screencast) gets a
   worktree and a target of its own too.
4. Keep scratch files (fixtures, GIFs, harnesses) in a folder named after the issue: parallel
   runs share a scratchpad and overwrite each other's `before.gif`.
5. Before a push, run what `.github/workflows/ci.yml` runs. A change a user can see adds its
   entry under `## [Unreleased]` in `CHANGELOG.md`, citing the issue as `(#N)`; docs, refactors,
   tests and tooling get none. What the change moves in the tests, the bullets at the top
   saying how:

   | The PR changes | It moves |
   |---|---|
   | a key or a flag | a tutor/drill task, smoke steps and their `keys.txt` |
   | what a screen draws | `tests/snapshots`, and the smoke screens it shows in |
   | a new screen, overlay or panel | its state in `STATES` |
   | what an edit key does | the `edit_fuzz` model |
   | where `d` lands | a fixture annotation, and the bench's baseline |
   | an `### Added` or `### Changed` entry | smoke steps citing `(#N)`, or a fixture annotation |
   | a bug | a test that fails without the fix |

   The `### Added` / `### Changed` row is enforced:
   `every_unreleased_feature_is_smoked_or_skipped_on_purpose` (`src/app/tests/smoke.rs`) fails on
   an entry no scenario or fixture cites, until its issue is listed in `UNSMOKED` with why.
6. Commits are in English and carry the reasoning. The repo squashes with the PR's commit
   messages, so a commit written with Claude keeps its `Co-Authored-By: Claude …` trailer.
7. Record the screencast, open the PR, and check `gh pr diff --name-only` holds only your files.
8. Once the whole task is in the PR, review it with Punchcard, once. Fix every finding on the
   branch's own change in this PR; a finding outside it becomes an issue. Do not review again:
   with the fixes pushed and CI green, go on as `## Merging` says. A docs-only PR needs no review.

Everything on GitHub (issues, PR bodies, reviews, comments) is in English.

## Changing `f`

`f` (`src/app/collapse.rs`) folds by the rules decided in #598, in the languages that pass the
fold bench and nowhere else: Python today, the rest in epic #623, each saying `no fold rules for
.EXT` until then. `tools/fold-bench/run` presses `f` on every line of real projects and compares
the fold with the reference the code's own syntax gives; a change to `f` keeps it at 100 % but
the lines its docstring names, and keeps `tests/folds/` green. A language turns on with a fold
fixture there and its row in the bench.

## Changing `d`

A wrong jump is worse than a picker or "don't know", and no lookup may get worse than on master.
Only the path the change narrows gets new rules: every other lookup, and every other language,
matches exactly as master does. Rules that start answering a new question tend to leak into the
fallback paths, so a `d` PR is ready to merge only after the bench shows no cursor worse than
master.

- **The bench**, `tools/d-bench/run [--lang go,rust]` (`tools/d-bench/README.md`): recorded
  cursors in real projects pinned to a commit, one per language, merl's answer scored against a
  language server's (or a judgement read from the code, where the README says so) and diffed
  against `baseline.tsv`, master's. It prints per language the direct hits, pickers with the
  answer, wrong jumps, misses and p50 / p90 ms, lists every cursor that got worse (a new wrong
  jump first), and exits 1 when there is any. Run the languages the change touches, all of them
  when a shared path moves; the table goes into the commit message. A PR that changes the table
  commits the new baseline with it (`--update-baseline`), so the next PR compares against what
  master will be.
- It times in release; the times mean something only with `sysctl -n vm.loadavg` under ~8:
  parallel sessions' builds push it to 30–90 and skew timings two- to threefold.
- The answers are recorded once, never in CI and never in merl (`record.py`); a cursor whose
  recorded answer is debatable is marked `skip` with the reason in `answers/LANG.tsv`, not argued
  with in the code.

The expected answers of `d` are written in the fixtures, under the line they probe
(`tests/fixtures/README.md`, #307). A fixed case flips its annotation from `today; want … (#N)`
to the wanted answer, or adds one when no annotation covers it; a miss found in a real project
becomes an annotation with the issue that will fix it.

An adversarial fixture per language (shadowed imports, namesake types, declaration-shaped lines
in strings) finds what real projects do not. To prove a test can fail, revert one fix at a time
and watch it go red.

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

- Bug fixes, precision work, refactors, tests and docs: merge your own PR and close its issue
  without asking, once CI is green and the findings of its one review are fixed.
- A change the user has to learn (a new or changed key, screen, animation or default): leave the
  PR open with the before/after screencasts, add the `needs-owner` label and name it in your
  status line. Never ask "can I merge?" in chat.
- The README and any other text in the owner's voice: open a draft PR with the `needs-owner`
  label and leave it to the owner.
- An issue holding a question only the owner can answer carries one of two labels until the
  answer is written into it:
  - `needs-owner`: the options are on the table (a recommendation, a before/after screencast
    from a prototype, the owner's earlier questions answered) and the owner only has to pick.
  - `to-think`: nobody has shaped it yet; the owner thinks it through before it is worked on. An
    agent that shapes one into a proposal (`.claude/skills/proposal/SKILL.md`) moves it to
    `needs-owner`.

  Once the decision is in the issue, the label goes and `agent-ok` comes, if nothing is left to
  ask. `is:open label:needs-owner` is the owner's queue of picks, PRs included;
  `is:open label:to-think` is the list to think over.
- `release-blocker`: an issue or PR the next release waits for: a crash, lost data, a regression
  since the last tag, a core flow (`--review`, `d`, editing) broken. Label it when you file or
  triage one; the owner may take it off. A `release-blocker` PR under `needs-owner` is the one
  the owner reads first.

`master` takes squash merges of PRs only, with the CI checks green; nobody can push to it
directly or bypass the checks. A branch behind master merges as it is: a queue of green PRs goes
in at once, and CI on the push to master catches two changes that break only together. A red
master is fixed before the next merge.

- A PR that conflicts with master gets no CI at all on a push. Merge `origin/master` into the
  branch, resolve, push: a merge keeps the commits a posted review links to, and the squash keeps
  it off master.
- In a stack, never `gh pr merge --delete-branch` a PR another one is based on: GitHub closes the
  upper PR for good. Merge the base, rebase the next branch onto `origin/master`, force-push,
  `gh pr edit --base master`. CI runs only for PRs into master, so push a new commit if it does
  not start after the retarget.

## Releases

Only when the owner asks for one, and with nothing open under `release-blocker`. In order:

1. The changelog covers every PR since the last tag (`git log vX.Y.Z..origin/master`): check
   each commit's issue and content, since PRs merge without an entry, and parallel merges leave a
   second `### Added` in `## [Unreleased]` to fold into the first. Dependabot, `docs:`,
   `refactor:` and tooling commits get no entry. Entries cite the issue. The entries go in on the
   release branch: its PR (step 4) carries them.
2. Smoke test: `.claude/skills/smoke-test/SKILL.md` (`/smoke-test` in Claude Code), from that
   branch, so the report reads the finished changelog. No release without its GO.
3. The release note, `docs/releases/X.Y.Z.md`: `release.yml` publishes it as the release, titled
   `merl X.Y.Z`, and fails without it. It is a list, not the changelog section:
   - `#### New` from `### Added`, `#### Better` from `### Changed`, `#### Fixed` from `### Fixed`;
     a heading with nothing under it is left out.
   - One line per changelog entry of the version, in the changelog's order: what you can do or
     what changed, the key or flag in backticks, the issue in parentheses. Up to ~12 words; no
     prose, no second sentence.
   - Under `#### Better` and `#### Fixed`, the `d` lines leave the list for a folded block at the
     end of their section, sorted so each language's lines stand together:
     `<details><summary>N more <code>d</code> fixes, by language</summary>` (`improvements`
     under Better). The owner asked for it on 0.8.0, whose 88 fixes were 63 `d` lines.
   - Last line, with the anchor GitHub builds from the version's heading (`## [0.8.0] -
     2026-09-27` is `#080---2026-09-27`); open the link to check it lands on the section:
     `` [Full changelog](https://github.com/Maksim-Burtsev/merl/blob/master/CHANGELOG.md#080---2026-09-27) · `brew upgrade merl` ``
   - No emoji, no intro paragraph, no contributor list. The model is the previous note in
     `docs/releases/`, or for the first one the sample in #272.
   - A GIF on top, above `#### New`, only when the release changes something visible that looks
     good in motion: `tools/cast.py --size 160x50` on a real project with the tree open. Show it
     to the owner and ask whether it goes in; without a yes the note has none. It goes to `media`
     as `releases/X.Y.Z.gif` (a new name for every upload, as for PR screencasts) and into the
     note as `![](https://raw.githubusercontent.com/Maksim-Burtsev/merl/media/releases/X.Y.Z.gif)`.
4. A release PR, `release: X.Y.Z`, from a branch `release/X.Y.Z`: only such a branch's CI
   presses `d`, `u` and `D` at every cursor of every fixture (#543, ~20 minutes), and a panic it
   finds is fixed on that branch. `## [Unreleased]` becomes `## [X.Y.Z] - YYYY-MM-DD` with its
   link, the version goes into `Cargo.toml` and `Cargo.lock`, it adds `docs/releases/X.Y.Z.md`,
   it carries the `tests/budgets.tsv` the smoke test's time budgets wrote, and its body holds the
   smoke test's verdict table.
5. After the merge, an annotated tag `vX.Y.Z` (message `merl X.Y.Z`) on that commit, pushed;
   `release.yml` builds the GitHub release and its binaries.
6. The Homebrew tap: `release.yml` bumps it when the `TAP_TOKEN` secret is set; otherwise bump
   `Formula/merl.rb` in `Maksim-Burtsev/homebrew-tap` by hand (the version and the four sha256
   of the `.sha256` assets), commit `merl X.Y.Z` and push. The first release that ships
   `merl-x86_64-apple-darwin.tar.gz` adds an `on_intel` block under `on_macos`, with that
   asset's url and sha256; `bump-tap` fails on an asset the formula has no block for.
