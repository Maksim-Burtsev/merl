---
name: groom
description: Go through merl's open issues with the owner, one at a time, until each is `agent-ok` or parked. Use when the owner wants to sort, triage or prepare issues (for a night run, for a release), asks what needs them, or asks which issues have questions.
---

# Grooming issues with the owner

The owner sits down for an hour and wants to leave with a batch of issues an agent can build
without them. Each issue costs them one read and one answer: what they decide is written into
the issue at once, so nobody asks again. `AGENTS.md` `## Merging` says what the labels mean.

## 1. Pick

The owner names the issues or a number of them; without either, take 10. Order: `needs-owner`
by priority, then open issues with no readiness label, then `to-think` the owner asked about.
Leave `in-progress` ones out. Read each with its comments: the owner's latest comment overrides
the body.

Show the list as one table and wait for the owner's word: number, one line in the user's terms,
`Visible`, how many questions you have. An issue with no question left is not discussed: say it
will get `agent-ok` and label it.

## 2. Prepare what needs a picture

An issue whose question is how something looks or moves gets its `## Proposal` first
(`.claude/skills/proposal/SKILL.md`), in the background while the others are discussed. The
owner does not pick a look from a description; such an issue is asked about once its pictures
are up, or stays `needs-owner` with the proposal for their next pass.

## 3. One issue per message

```
#N <title> — <link>
<what it is, one sentence: what the user presses and sees today>
Visible: <no | what changes on screen>

q1. <the question>
  a) <recommended option> — <one reason>
  b) <option>
  GitHub / GitLab / VS Code: <what they do, when that answers it>
q2. …
```

- Every question has options and the recommended one is `a`. A question you can answer from
  the code, the forges or an earlier decision is not a question: answer it and say so in a line.
- The user's terms: keys, screens, messages. Code only when the owner asks how.
- The owner answers like `q1 a, q2 ok`. `ok` is your recommendation. A new question in their
  answer is answered before moving on.

Go to the next issue when every question of this one has an answer or the owner parks it.

## 4. Write it down, then move the label

Before the next issue: the decisions go into the issue as a `## Decided` section at the top of
the body (the question, the pick, the owner's reason when they gave one), in English. Then:

- nothing left to ask, and nothing visible without a picked proposal: `agent-ok`;
- parked: `backlog`, with the reason in a comment;
- still open: stays `needs-owner`, the open question first in the body.

## 5. Close

One table: what became `agent-ok`, what waits for the owner and for what, what was parked. The
owner starts the build themselves (`.claude/skills/batch/SKILL.md`).
