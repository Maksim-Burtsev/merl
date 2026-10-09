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

Fill the hours: a small fix is ~15 minutes of one agent, an ordinary issue ~30, a large one ~60;
two agents work at once at night and four by day (see **Calm**), and an epic costs ~1.5 hours
on top for integration, its review and CI. Correct these from the ledgers of earlier runs when there are any.

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
- `ledger.md` there, a row per issue: branch, agent, state, tokens, minutes. Keep it current
  after every agent result: a usage limit can end the run at any moment, and the ledger is what
  the next session resumes from.
- Label every picked issue `in-progress`, with a comment naming its branch.
- One release build of origin/master in a detached worktree, for every "before" screencast.
- A worktree per task, made by you: `tools/worktree ../merl-b<date>-<N> <branch>`. Agents get the path; they do not make worktrees or use `isolation`.
- The epic: a branch `epic/agent-ok-<date>` from origin/master.

## 4. Run

Each implementer is one agent whose prompt is two lines: read `.claude/skills/batch/implementer.md`
and its task file (kind, issue, worktree, branch, base, scratch folder, the master build, the
commit trailers).

- **Calm.** At night at most 2 agents at once, by day 4: a night has hours to spare, and more
  agents only queue on the Mac mini's 10 cores and run its fan all night. The owner may lift it
  for a run ("без ограничения", a big batch to finish by morning): then 4, and the task files say
  so. While `sysctl -n vm.loadavg` reads over 16, start nothing. Other sessions share the
  machine: their processes are theirs.
- **Epic issues.** The agent pushes its branch: no PR, no review. Read its diff, merge the
  branch into the epic (`--no-ff`), and check each conflict hunk by hand: fixes for different
  languages add helpers at the same spot, and a careless union drops a closing brace. When
  several branches wait, a separate integrator agent keeps the conflicts out of your context.
  After each merge of a language or of master, `tests/smoke/run.py --golden` on the epic.
- **Own-PR issues.** The agent opens its PR, with before/after screencasts when it is visible
  (then under `needs-owner`), and reviews it with Punchcard once itself. The finders run in the
  foreground inside the implementer (`run_in_background: false`), so the review comes back to
  it and not to you.
- **The epic PR.** One PR into master, a section per issue (before, after, the test that catches
  it). One fresh agent runs Punchcard on it once and its findings are fixed in one pass, by
  parallel agents on branches off the epic when there are many. No second review. A finding
  outside the batch becomes an issue.
- **The owner's call.** A Punchcard finding that asks whether a change is the owner's call is
  never answered by you or an implementer: that change leaves the epic or the PR, into its own
  PR under `needs-owner` with a before/after screencast, or it is reverted. On 2026-10-02 a
  dispatcher answered one itself and shipped a visible change to `d` in C++ (#373) inside an
  invisible epic.
- **A stuck agent.** A blocked agent sends no notification: on 2026-10-03 two night agents
  waited 4–6 hours on a permission prompt while listed as working. On every wake, and at least
  every 30 minutes (a Monitor or ScheduleWakeup), read the modification time of each running
  agent's output file. An agent silent for more than 20 minutes is named in the chat with its
  last command; unblock it, or stop it and push its worktree's commits. Read file times only,
  never the machine's load.
- A bug met on the way is filed with its three labels, not fixed in passing.

## 5. Left-over time

Shape issues for the owner's next pass: a `## Proposal` for `to-think` issues and for
`needs-owner` ones that have none (`.claude/skills/proposal/SKILL.md`), one agent each, the
highest priority first.

## 6. Close

- Merge the epic when it is green, unless the owner said in the session that it waits; then it
  waits under `needs-owner`. One consolidated squash message, a paragraph per issue.
- Remove the worktrees and local branches of everything merged or handed over as a PR, the
  master build included. Take `in-progress` off whatever was not finished, with a comment on
  what is left.
- The last message is one table, in the owner's language: merged (issue, one line), waiting for
  the owner (link, what to look at), not done (why, what is left), then tokens and hours against
  the estimate.
