---
name: batch
description: Build a batch of merl's `agent-ok` issues without the owner, as the dispatcher of implementer agents: an evening queue or a night run. Use when the owner asks to take, pick or run several issues, to leave work for the night, or to go on with the queue.
---

# A batch of `agent-ok` issues

You are the dispatcher: you pick, brief, integrate and report; implementer agents build, one
per issue. The owner is away once the run starts, so every question is asked before it, and
whatever needs them afterwards waits under `needs-owner`.

## 1. Ask, once

Ask nothing the repo already answers: invisible work merges when it is green (AGENTS.md
`## Merging`). In one message, ask only what the owner has not said in the session:

- About how many hours of work to pick?
- The budget: a share of the weekly limit, and until 2026-10-22 a token budget beside it (the
  owner's form: "10 % of the week, or 4M"). The account's usage endpoint, undocumented and in
  whole percents, gives the weekly percentage:
  `curl -s https://api.anthropic.com/api/oauth/usage -H "Authorization: Bearer $(security
  find-generic-password -s 'Claude Code-credentials' -w | jq -r .claudeAiOauth.accessToken)" -H
  'anthropic-beta: oauth-2025-04-20' | jq .seven_day`. Read it at the start, into the ledger, and
  on every wake: start no new agent once `utilization` has risen by the budget since the start,
  or reads 95. When it does not answer, go by the token budget, counted as before. It counts the
  whole account, other sessions included.
- The kind of run, when the owner did not name it:

| Kind | Shape |
|---|---|
| **Night run**: many tasks, owner away | every invisible task goes into one epic branch, and the epic gets one Punchcard as a whole; a Punchcard per branch once spent ~30 % of a weekly limit in a night |
| **Day stream**: 5–10 small tasks, about an hour | each task is its own PR into master, reviewed with Punchcard once, fixed, merged when green |
| **Release preparation** | tasks gather in an epic, as in a night run |

The kind is a default, not a law. What the owner says in the session wins over it and over any
memory of an earlier run's mode: on 2026-10-02 a dispatcher followed a remembered "own PR" mode
against the owner's "all in one epic" and spent a weekly limit on 18 reviews.

## 2. Pick and show

`gh issue list --search 'is:open label:agent-ok -label:epic -label:in-progress'`, `P1` first,
then `P2`, `P3`. Read each with its comments. Leave out an issue with a question still open
(move it to `needs-owner`) and a visible one without a picked `## Proposal`.

Fill the hours with these minutes of one agent, measured in the ledgers of 2026-10-02..10-15:

| Task | Minutes |
|---|---|
| A small fix: one rule, one key | 7–25 |
| A lint or comment slice | 25–45 |
| A proposal | 9–15 |
| A grammar | about 60 per language |
| An own visible PR, screencasts and Punchcard included | median about 55 |

Three agents work at once at night and four by day (see **Calm**), and an epic costs ~1.5 hours
on top for integration, its review and CI, while the next tasks go on building (**While the
epic closes**). Correct these from the ledgers of earlier runs when there are any.

Show one table and wait for the owner's word to start:

| # | What, in one line | Visible | Goes to | Estimate |
|---|---|---|---|---|

`Goes to`, per task, is where the run's kind sends it, so a change of mode is a visible change
of plan:

- `epic`: an invisible task in a night run or a release preparation;
- `own PR`: an invisible task in a day stream, merged when green;
- `needs-owner`: a change of a key, screen, animation or default, in any kind of run: its own
  PR with a before/after screencast, left for the owner.

Issues that touch the same code (one language's `d` rules) share a branch and an agent, in
sequence.

## 3. Prepare

- A folder for the run, `~/.cache/merl-batch/<date>/`: the ledger, one prompt file per task,
  scratch folders per issue.
- The owner's words from the session (the kind of run, what merges, what waits, anything said
  against the skill's default) go into the run's files before the first agent starts: a
  `COMMON.md` every task file points to, so implementers and a resuming session read them.
- `ledger.md` there: the weekly percentage at the start, then a row per issue: branch, agent,
  state, tokens, minutes. A merged `epic` row whose issue keeps work starts its State with
  `part:` (`part #874:` when only some of the row's issues do): `epic-body` writes those as
  `Part of`, every other merged issue as `Closes`. Keep it current
  after every agent result: a usage limit can end the run at any moment, and the ledger is what
  the next session resumes from.
- Label every picked issue `in-progress`, with a comment naming its branch.
- One release build of origin/master in a detached worktree, for every "before" screencast.
- A worktree per task, made by you: `tools/worktree ../merl-b<date>-<N> <branch>`. Agents get the path; they do not make worktrees or use `isolation`.
- The epic: a branch `epic/agent-ok-<date>` from origin/master.

## 4. Run

Each implementer is one `merl-implementer` agent (`.claude/agents/`) whose prompt is two lines:
read `.claude/skills/batch/implementer.md` and its task file (kind, issue, worktree, branch, base,
scratch folder, the master build, the commit trailers). Integrators, fixers and the epic's
reviewer are `merl-implementer` too, and every Punchcard finder is a `merl-finder`: both skip the
owner's CLAUDE.md, memory and connectors: an implementer starts at ~18k tokens and a finder at
~6k, where a `general-purpose` agent starts at ~54k.

- **Calm.** Every build, test, clippy, `ci-local`, smoke run, bench and sweep, yours and the
  agents', runs under `tools/heavy`: two at a time on the whole machine, 5 jobs and 5 test
  threads each, so the Mac mini's 10 cores stay busy and its fan calm, and an agent that reads,
  writes or waits on a review holds no slot. At night 3 agents at once, by day 4; the owner may
  lift it for a run ("без ограничения", a big batch to finish by morning): then 4 at night too,
  and the task files say so. While `sysctl -n vm.loadavg` reads over 16, start nothing. Other
  sessions share the machine: their processes are theirs. The slots are on trial from the night
  of 2026-10-15: note in the ledger's last line the load you saw and how long agents waited for
  a slot.
- **Epic issues.** The agent pushes its branch: no PR, no review. Read its diff, merge the
  branch into the epic (`--no-ff`), and check each conflict hunk by hand: fixes for different
  languages add helpers at the same spot, and a careless union drops a closing brace. A
  conflict on `EXPECTS` or `LONG_FILES` in `src/main.rs` is measured again, never unioned:
  `.claude/skills/batch/bin/recount` in the worktree mid-merge, once its other conflicts are
  resolved, runs the two tests that own them and writes and stages the measured values. When
  several branches wait, a separate integrator agent keeps the conflicts out of your context.
  After each merge of a language or of master, `tests/smoke/run.py --golden` on the epic.
  Before merging, `git diff --stat EPIC...BRANCH` lists only files the task needs: an agent
  that squashed or rebased on a stale tree can roll other files back without a conflict, and
  on 2026-10-17 a Ruby branch put back `src/git.rs` from before #973's guard against deleting
  the user's files.
- **Own-PR issues.** The agent opens its PR, with before/after screencasts when it is visible
  (then under `needs-owner`), and reviews it with Punchcard once itself. An invisible PR then
  gets `gh pr merge --squash --auto`: GitHub merges it when CI is green, and nobody waits on CI.
  On every wake, `gh pr list --author @me --json number,autoMergeRequest,statusCheckRollup`
  shows a red one: a fixer agent pushes the fix to its branch. The finders run as
  `merl-finder` in the foreground inside the implementer (`run_in_background: false`), so the
  review comes back to it and not to you.
- **The epic PR.** First the full `tools/heavy tools/ci-local` on the epic: its tasks ran only
  `clippy tests`. Then one PR into master, a section per issue (before, after, the test that
  catches it): `.claude/skills/batch/bin/epic-body [RUN_DIR]` drafts `epic-body.md` and
  `squash-msg.txt` in the run's folder from the ledger's merged `epic` rows, each
  implementer's report and `COMMON.md`'s trailers; read every section, merge the ones of one
  issue and check `Closes` against what the reports say is left. One fresh agent runs Punchcard on it once and its findings are fixed in one pass,
  by parallel agents on branches off the epic when there are many. No second review. A finding
  outside the batch becomes an issue. With the fixes pushed, `gh pr merge --squash --auto`, and
  one background `gh pr checks --watch` on it; a red check gets a fixer agent.
- **While the epic closes.** Tasks still waiting do not wait for it: from the moment the epic PR
  is opened, they go into `epic/agent-ok-<date>-2`, cut from the epic, with the free agents.
  Merge the epic's last commit into it before the squash, and origin/master after. Lines the
  second epic changed again conflict with the squash; take each such file as
  `git merge-file OURS EPIC'S-LAST THEIRS` (the first epic's last version as the base), and
  check that only the files master changed after the first epic moved. It then closes as an
  epic of its own.
- **The owner's call.** A Punchcard finding that asks whether a change is the owner's call is
  never answered by you or an implementer: that change leaves the epic or the PR, into its own
  PR under `needs-owner` with a before/after screencast, or it is reverted. On 2026-10-02 a
  dispatcher answered one itself and shipped a visible change to `d` in C++ (#373) inside an
  invisible epic.
- **A stuck agent.** A blocked agent sends no notification: on 2026-10-03 two night agents
  waited 4–6 hours on a permission prompt while listed as working. On every wake, and at least
  every 30 minutes (a Monitor or ScheduleWakeup), read the modification time of each running
  agent's output file: `.claude/skills/batch/bin/stuck [TASKS_DIR]` prints each running agent
  of this session with its silent minutes and last call, then the load and the time (`sleep
  1800; …/stuck` in the background wakes you). An agent silent for more than 20 minutes is named
  in the chat with its last command; unblock it, or stop it and push its worktree's commits.
  Judge by file times only, never by the machine's load.
- A bug met on the way is filed with its three labels, not fixed in passing.

## 5. Left-over time

Shape issues for the owner's next pass: a `## Proposal` for `to-think` issues and for
`needs-owner` ones that have none (`.claude/skills/proposal/SKILL.md`), one agent each, the
highest priority first.

## 6. Close

- The epic merges itself when it is green (`--auto`, above), unless the owner said in the
  session that it waits; then it waits under `needs-owner`, with no auto-merge. One consolidated
  squash message, a paragraph per issue (`epic-body`'s `squash-msg.txt`, read and trimmed):
  `gh pr merge --squash --auto --body-file`.
- Once the epic is on master, `.claude/skills/batch/bin/refresh-prs [-n] [RUN_DIR]` merges
  origin/master into each open `needs-owner` PR the ledger names (a merge, not a rebase:
  reviews link to the commits), pushes the clean ones and lists the conflicting ones; each of
  those goes to an agent that resolves it, regenerates screens and snapshots, and runs
  `tools/ci-local` before its push. `-n` merges without pushing.
- Remove the worktrees and local branches of everything merged or handed over as a PR, the
  master build included. Take `in-progress` off whatever was not finished, with a comment on
  what is left.
- With every PR pushed and no agent running, `.claude/skills/batch/bin/clean-builds [RUN_DIR]`
  deletes the run's build output: each cargo `target` dir (3–10 GB) under the run's folder and
  this session's `/private/tmp` folder. The ledger, task files, logs and reports stay. On
  2026-10-18 runs that never did this had left ~100 GB and filled the disk.
- The last message is one table, in the owner's language: merged (issue, one line), waiting for
  the owner (link, what to look at), not done (why, what is left), then the weekly percentage
  and the tokens used, and the hours, against the budget and the estimate.
