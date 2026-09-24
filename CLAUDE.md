# merl — notes for agents

- `src/app/mod.rs` `KEYS` is the single source of truth for the key bindings: the `?` overlay, the
  README `## Keys` table (a test compares them) and the tutorial all derive from it.
- `merl --tutor` (`src/tutor.rs`, sample project in `tutor/notes/`) walks `TUTOR`, a list of
  tasks from `POOL` (`src/tutor/pool.rs`), which the drill (#209) shares: one task per action of
  `KEYS`, each with a start of its own, a tutor text that names the key, a drill text that never
  does, and the answer that does it. When you add, remove or rebind a key, or change what an
  overlay or jump does, update the affected task or add one. The test
  `every_key_is_taught_or_skipped_on_purpose` fails on a new `KEYS` action until a task trains it
  or it is listed in `NOT_TAUGHT`; `every_task_starts_undone_and_its_answer_does_it` fails when a
  task's start already does it or its answer does not, from cold or with the whole pool run
  before it on one App.

## Pull requests

- The body of a pull request is read by a merl user, not by a reviewer of the diff. It is the
  shape of `.github/pull_request_template.md` and nothing more: the issue it closes, one
  sentence on what happens today, one on what happens now, the before/after screencasts, and a
  `Keys —` line that is always there (`none` when nothing moved). A bullet list only when more
  than one thing is visible; for a single change it would repeat **Now**. Everything else — why
  this design, what was tried, the rules per language — belongs in the commit messages.
- Anything visible in the interface ships with a before/after screencast. `tools/cast.py` records
  one run of one binary as a GIF; run it twice with the same steps file, once against a build of
  master (its own worktree, its own `CARGO_TARGET_DIR`), once against the branch. Record outside
  the checkout (`cp -R tutor/notes /tmp/notes`), keep a run under ~15 s, and push the two GIFs to
  the orphan `media` branch under `prs/`. Internal refactoring, docs and tests need none.
