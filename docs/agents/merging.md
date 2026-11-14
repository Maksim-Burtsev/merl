# Merging

- Bug fixes, precision work, refactors, tests and docs: merge your own PR and close its issue
  without asking. Once the findings of its one review are fixed, `gh pr merge --squash --auto`:
  GitHub merges it when CI is green, and nobody waits on CI. A red check is yours to fix.
- A change the user has to learn (a new or changed key, screen, animation or default): leave the
  PR open with the before/after screencasts, add the `needs-owner` label and name it in your
  status line. Never ask "can I merge?" in chat.
- The owner's comment `ok` on a PR merges it through `.github/workflows/owner-ok.yml`, at the
  commit it was given to; a push after it needs a new `ok`. Any other owner comment is a brief.
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
