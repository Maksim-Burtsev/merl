---
name: smoke-test
description: The release gate of merl, step 2 of AGENTS.md `## Releases`, after the changelog. Use whenever a merl release is asked for or prepared (a version bump, a tag, a `release: X.Y.Z` PR), and to check a build against the last release. Plays the tests/smoke scenarios in tmux on both, compares their screens and timings, and ends in a GO / NO-GO verdict table for the release PR.
---

# Smoke test before a release

The run answers two questions: did anything get worse than in the last release, and does what
the changelog promises work. A release goes out only on a GO. Its per-PR half runs in CI:
`run.py --golden` plays the scenarios on the PR's build alone against the screens checked in
under `tests/smoke/screens/`, as text, without timings or the last release.

## 1. Run

From a worktree of the release branch, the changelog finished (step 1), with its own `target/`:

```sh
sysctl -n vm.loadavg          # timings mean something under ~8: write it down with them
tests/smoke/run.py            # ~2-3 min: build, fetch the last release, play, report
```

`run.py` builds the checkout (`cargo build --release --locked`), downloads the last GitHub
release's binary once into `~/.cache/merl-smoke/`, rebuilds the fixture (`tests/smoke/setup.sh`)
and the README's gitea and polar (`assets/tapes/setup.sh`, fetched once) under
`/tmp/merl-smoke`, plays every `tests/smoke/*.steps` on the new build, then on the old one, and writes
`/tmp/merl-smoke/out/report.md` with a PNG per differing checkpoint beside it (old above new).
Needs macOS, tmux, Go and Python 3 (`d` goes into their standard libraries), Pillow, and
`gh` logged in. `--help` has the scenario grammar. One run at a time: a second one is refused
while the first holds `/tmp/merl-smoke`.

When the change under test touches `run.py` itself, run `tests/smoke/run.py --selftest` first:
fake merls go through every verdict in about 20 seconds, and it ends `selftest ok`.

The run is done when the report's table has a row for every scenario.

## 2. Read the report

- **The table**, its legend under it: PASS needs nothing; DIFF has differences to judge. FAIL (a
  wait timed out), CRASH (a panic or a signal), EXIT, HUNG (would not quit) and RUN (a `run`
  step failed: wrong bytes on disk, a save lost) in the `new` column are the new build failing
  the scenario, in any of its plays; `old` failing where `new` passes is usually a fix. ERROR is
  the runner failing, not merl: rerun, and fix `run.py` if it comes back. Rerun a red scenario
  alone (`tests/smoke/run.py --only NAME`, its report in `/tmp/merl-smoke/out-only`, the full
  one left in place) before you call it: a real one comes back, a flake of the load does not.
- **Differences**, each once with every checkpoint it shows in: open the PNG of each, since
  the text diff misses what only colour or position shows.
- **Timed steps**: a step marked **slower** (over 2× and over 200 ms slower) is a regression
  unless an Unreleased entry says why. Samples far apart mean load: rerun under ~8.
- **Plays that did not end ok**, each with its stderr and a PNG of the screen at the failure (for
  a merl that died, its last checkpoint).
- **Unreleased entries**, with the scenarios whose comments cite their issues.

## 3. Look at every checkpoint

The report compares screens as text: colour, contrast and alignment in text that did not change
never reach it (#144's deleted lines nearly invisible, #146's 1.6:1 contrast were found by
people). So every checkpoint of the new build gets looked at, in two themes:

```sh
tests/smoke/run.py --all-shots   # ~2 min: the new build alone, every scenario in each theme
```

It writes `/tmp/merl-smoke/shots/shots.md`, which lists the PNGs beside it:
`default/SCENARIO/NN.png` in merl's default theme on a dark terminal, `github-light/SCENARIO/NN.png`
in a light theme on a light one, each screen headed by its checkpoint. A play that did not end ok
is named there with its `last.png`; step 2 already judges it. Open every PNG and look for:

- text cut at an edge, of the screen or of a panel;
- panels overlapping;
- contrast too low to read: text, a deleted line, a selection, the cursor line, a hint;
- the cursor off its word;
- a state drawn in the wrong colour: an added line in red, a status letter, an error.

A finding is a regression only when the last release does not have it. Look at the same
checkpoint on it: `tests/smoke/run.py --all-shots --new ~/.cache/merl-smoke/TAG/merl --only NAME
--out /tmp/merl-smoke/shots-old` (step 1 fetched it; it stops at the first wait for what it
lacks, so a checkpoint it never reaches is new). A regression goes to step 4 as one. Every other
finding is an issue and does not block: search the open ones first (`gh issue list --search`),
else file a `bug` per AGENTS.md `## Issues`, its PNG uploaded to `media` as
`issues/<slug>.png` (a name not taken yet) and the scenario's steps up to the checkpoint.

## 4. Give the verdict

Every difference, failure and slower step gets one verdict:

- `intended (#N)`: an Unreleased entry describes this change; cite its issue.
- `changelog gap`: a visible change the changelog does not mention, wanted all the same. The
  entry goes into the release PR.
- `regression`: anything else a user would see as worse, a FAIL, CRASH, EXIT, HUNG or RUN of
  the new build, a slower step no entry explains, a step 3 finding the last release does not
  have.

Every Unreleased entry gets one too: `seen working in SCENARIO` when a scenario cites its issue,
passed on the new build and shows the change in a checkpoint, else `not checked`. `not checked` does not
block; nobody tries entries by hand for now (#234, decision 4).

**GO** needs zero regressions and zero crashes. A **NO-GO** means no release: file an issue per
regression, labelled `bug`, with the two screens (the PNG) and the keys that repeat it (the
scenario's steps up to the checkpoint). A bug the last release has too gets its own issue and
does not block.

## 5. The verdict table

It goes into the body of the release PR, a row for every scenario and every Unreleased entry:

```markdown
Smoke test: merl 0.8.0 (abc1234) against v0.7.0, 16 scenarios, wall 150 s, load 3.1

| | verdict | evidence |
|---|---|---|
| scenario `review` | intended (#165) | differences 1-3: the diff tints, review/04.png |
| scenario `edit` | PASS | |
| timed steps | none slower | `d` on gitea 48 ms / 51 ms |
| every checkpoint, two themes | 2 findings, not regressions | 212 PNGs; #460, #461 filed |
| #165 review paints the diff as GitHub does | seen working in `review` | |
| #227 `d` on `Type::name` in Rust | not checked | no scenario reads Rust |

**GO**: 0 regressions, 0 crashes.
```

The skill is done when that table has a verdict in every row and ends in GO or NO-GO.
