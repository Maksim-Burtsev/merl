---
name: batch
description: Build a batch of merl's `agent-ok` issues without the owner, as the dispatcher of implementer agents: an evening queue or a night run. Use when the owner asks to take, pick or run several issues, to leave work for the night, or to go on with the queue.
---

# A batch of `agent-ok` issues

You are the dispatcher: you pick, brief, integrate and report; implementer agents build, one
per issue. The owner is away once the run starts, so every question is asked before it, and
whatever needs them afterwards waits under `needs-owner`.

## 1. Ask, once

Two questions, in one message:

- About how many hours of work to pick?
- May the invisible part be merged when it is green, or does everything wait for the owner?

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

`Goes to` is `epic` for a fix nobody has to learn, and `own PR` for a change of a key, screen,
animation or default. Issues that touch the same code (one language's `d` rules) share a
branch and an agent, in sequence.

## 3. Prepare

- A folder for the run, `~/.cache/merl-batch/<date>/`: the ledger, one prompt file per task,
  scratch folders per issue.
- `ledger.md` there, a row per issue: branch, agent, state, tokens, minutes. Keep it current
  after every agent result: a usage limit can end the run at any moment, and the ledger is what
  the next session resumes from.
- Label every picked issue `in-progress`, with a comment naming its branch.
- One release build of origin/master in a detached worktree, for every "before" screencast.
- A worktree per task, made by you: `git worktree add ../merl-b<date>-<N> -b <branch>
  origin/master`. Agents get the path; they do not make worktrees or use `isolation`.
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
- **Own-PR issues.** The agent opens its PR with before/after screencasts under `needs-owner`
  and reviews it with Punchcard once itself.
- **The epic PR.** One PR into master, a section per issue (before, after, the test that catches
  it). One fresh agent runs Punchcard on it once and its findings are fixed in one pass, by
  parallel agents on branches off the epic when there are many. No second review. A finding
  outside the batch becomes an issue.
- **An overdue agent.** `ListAgents`; a hung one is stopped and its worktree's commits pushed.
- A bug met on the way is filed with its three labels, not fixed in passing.

## 5. Left-over time

Shape issues for the owner's next pass: a `## Proposal` for `to-think` issues and for
`needs-owner` ones that have none (`.claude/skills/proposal/SKILL.md`), one agent each, the
highest priority first.

## 6. Close

- Merge the epic when it is green, if the owner allowed it; otherwise it waits under
  `needs-owner`. One consolidated squash message, a paragraph per issue.
- Remove the worktrees and local branches of everything merged or handed over as a PR, the
  master build included. Take `in-progress` off whatever was not finished, with a comment on
  what is left.
- The last message is one table, in the owner's language: merged (issue, one line), waiting for
  the owner (link, what to look at), not done (why, what is left), then tokens and hours against
  the estimate.
