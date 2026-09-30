---
name: proposal
description: Shape a visible merl change into a `## Proposal` the owner can pick from: a still per option, real demos, what GitHub, GitLab and VS Code do, one recommendation. Use when an issue changes a key, screen, colour, animation or default and has no accepted proposal yet, when moving a `to-think` or `needs-owner` issue forward, and when the owner asks to see options or how something will look.
---

# A proposal the owner picks from

The owner decides a visible change by eye, from the issue itself: a `## Proposal` at the top of
its body. They accept a change only when it is plainly better and nothing ordinary got worse, so
the proposal answers their three standing questions before they ask:

- What does each option look like in real use, on a real project?
- What is the rule: when does the new drawing or behaviour kick in, and when not?
- Does the ordinary flow stay exactly as it is?

Done when the issue's body opens with the section below, every link in it opens, and the issue is
under `needs-owner`.

## 1. Read

The issue with its comments: the owner's latest answer overrides the body. Then find out what
GitHub, GitLab and VS Code do in the same situation, from their own pages or source, not from
memory. For review mode their shared behaviour is the default proposal: where the two forges
differ, propose what both share; bring an option of merl's own only when it is clearly better,
and show the difference.

## 2. Options

At most 4, each a rule of one or two lines in the user's terms (what is on screen, what a key
does). One is recommended. An option that adds an element to the screen (an icon, a count, a
colour, a line of text) says so in its rule: the owner weighs every addition.

## 3. Prototype

In a worktree of your own, detached at origin/master, with its own `target/` (`AGENTS.md`,
`## Working on an issue`). One release build serves every option: an environment variable or a
hidden flag switches between them. The prototype is throwaway: never pushed, no PR, the worktree
removed when the proposal is up.

## 4. Pictures

Stills carry the decision; the owner compares options side by side and will not watch a GIF per
option per theme.

- **A still per option**, and one of **now**: `tmux capture-pane -e -p` into `tools/shot.py`, on a
  real project cloned into your scratch folder and copied to `/tmp/<N>-<name>` (`media` is
  public: no home directory, no scratch path in a frame). The pane is a laptop's full screen,
  160x50, in `tokyonight-moon`.
- **The ordinary flow**: a still of an everyday project where the change must not kick in, for
  the recommended option, beside the same screen on master.
- **A light theme**: the recommended option once more in `tokyonight-day`
  (`merl -t tokyonight-day`, `tools/shot.py … 'TokyoNight Day'`).
- **A colour change**: a sheet of 6 themes, 3 dark and 3 light, for the recommended option. A
  colour that fits one theme and jars in another is how the A / M / D letters were shipped and
  removed (#256, #498).
- **A GIF only where motion is the point** (scrolling, a jump, an animation): `tools/cast.py`,
  one per option, moving as `AGENTS.md` `## Screencasts` says.

Open every picture yourself before uploading: the frame shows the change, in the right theme,
without a path of yours.

Upload to `media` as `issues/<N>-<slug>-<variant>.<ext>`, each name free first (`AGENTS.md`,
`## Screencasts`).

## 5. Write it into the issue

Prepend to the body, or replace a `## Proposal` already there; the old body stays below the rule.
English, no implementation talk.

```
## Proposal

**Now.** <one line> ![now](PNG)

**A. <name> (recommended).** <the rule>. <why, one reason>
![A](PNG) · [in motion](GIF)

**B. <name>.** <the rule>
![B](PNG)

**Elsewhere.** GitHub: <what it does>. GitLab: <…>. VS Code: <…>.

**Unchanged.** <the ordinary flow, one line> ![ordinary](PNG) · light theme ![light](PNG)

---
```

Then move the issue from `to-think` to `needs-owner`. Report in a few lines: the issue's link,
the options by their rules, the recommendation, and anything the demos revealed.
