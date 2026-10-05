---
name: proposal
description: Shape a visible merl change into a `## Proposal` the owner can pick from: every case it touches, a real screencast of merl per option, options copied from the classic terminal editors, one recommendation. Use when an issue changes a key, screen, colour, animation or default and has no accepted proposal yet, when moving a `to-think` or `needs-owner` issue forward, and when the owner asks to see options or how something will look.
---

# A proposal the owner picks from

The owner decides a visible change by eye, from the issue itself: a `## Proposal` at the top of
its body. They accept a change only when it is plainly better and nothing ordinary got worse, so
the proposal answers their three standing questions before they ask:

- What does each option look like in real use, on a real project, in merl itself?
- What is the rule: when does the new drawing or behaviour kick in, and when not?
- Does the ordinary flow stay exactly as it is?

Done when the issue's body opens with the section below, every link in it opens, and the issue is
under `needs-owner`.

## 1. Read

The issue with its comments: the owner's latest answer overrides the body. Then find out what
the classic terminal editors do in the same situation: Vim (netrw, NERDTree), Neovim (nvim-tree,
neo-tree, LazyVim's explorer), Emacs (dired, treemacs), Helix, and terminal tools such as lazygit
where they fit; GitHub and GitLab for anything review mode shows. Read their docs or source, run
them where you can, and record them on the same project as merl: their default is the standard
an option copies one to one or adapts, never an invention of merl's own (#399). For review mode their shared behaviour is the default proposal: where the two forges
differ, propose what both share; bring an option of merl's own only when it is clearly better,
and show the difference.

## 2. Cases

Before any option, list every case the change touches, not only the one in the issue: each kind
of row or element it draws, the cursor on it and off it, the tree or the code focused, the
project tree and the review panel, a narrow terminal (80×24) and a wide one, an ordinary project
where it must not kick in. The list goes into the proposal as a table, and every option is
recorded on all of it. Anything drawn over another pane is checked frame by frame where it meets
that pane's text: #399's first build showed one case, and its cursor row ran into the code.

## 3. Options

At most 4, each a rule of one or two lines in the user's terms (what is on screen, what a key
does), naming the tool it copies and what it adapts. One is recommended. An option that adds an element to the screen (an icon, a count, a
colour, a line of text) says so in its rule: the owner weighs every addition.

## 4. Prototype

In a worktree of your own, detached at origin/master, with its own `target/` (`AGENTS.md`,
`## Working on an issue`). One release build serves every option: an environment variable or a
hidden flag switches between them. The prototype is throwaway: never pushed, the worktree removed
when the proposal is up, unless the owner asks for a PR (end of step 5).

## 5. Pictures

Every picture is merl itself: a real build, run in tmux on a real project, recorded with
`tools/cast.py`. The owner must be able to run the same steps and see the same screen, one to one.

- **A screencast per option and case**, and one of **now** on master: the same steps file for each, moving
  as `AGENTS.md` `## Screencasts` says. The project is cloned into your scratch folder and copied
  to `/tmp/<N>-<name>` (`media` is public: no home directory, no scratch path in a frame).
- **One size and font for the whole proposal**: `--size 160x50`, a laptop's full screen, unless
  the issue is about a width; cast.py's default font and the Ghostty TokyoNight Moon colours.
- **The issue shows the GIF itself**, inline, never a still with a link to it: the owner watches
  the option, not a frame of it (#720). A still, a frame taken out of that screencast
  (`Image.open(gif).seek(n)`), is for the decision desk only. One screen per picture: two screens
  side by side are two pictures.
- **The ordinary flow**: a screencast of an everyday project where the change must not kick in,
  for the recommended option, beside the same steps on master.
- **A light theme**: the recommended option once more in `tokyonight-day` (`merl -t
  tokyonight-day`, `--ghostty 'TokyoNight Day'`).
- **A colour change**: the recommended option in 6 themes, 3 dark and 3 light. A colour that fits
  one theme and jars in another is how the A / M / D letters were shipped and removed (#256,
  #498).
- **How to see it yourself**, under each picture: the project and its commit, the binary (master
  at its sha, or the prototype's switch), the steps file.

Before uploading, open each still and the live `tmux capture-pane -p` of the same moment: the
same text in the same cells, the change in view, the right theme, no path of yours.

When the owner cannot pick from pictures, build the recommended option as one PR. Its body shows
now and every option as screencasts of the prototype's build, the recommendation, and asks for one
letter. The other options never become PRs of their own: the owner reads one PR per issue.

Upload to `media` as `issues/<N>-<slug>-<variant>.<ext>`, each name free first (`AGENTS.md`,
`## Screencasts`).

## 6. Write it into the issue

Prepend to the body, or replace a `## Proposal` already there; the old body stays below the rule.
English, no implementation talk.

```
## Proposal

**Now.** <one line>

![now](GIF)

**A. <name> (recommended).** <the rule>. <why, one reason>

![A](GIF)

To see it: <project@commit, binary, steps>

**B. <name>.** <the rule>

![B](GIF)

To see it: <…>

**Elsewhere.** GitHub: <what it does>. GitLab: <…>. VS Code: <…>.

**Unchanged.** <the ordinary flow, one line>

![master](GIF) ![recommended](GIF)

Light theme: ![light](GIF)

---
```

Then move the issue from `to-think` to `needs-owner`, and put its question on the decision desk
(`.claude/skills/groom/desk.md`) with the same stills and screencasts. Report in a few lines: the issue's link,
the options by their rules, the recommendation, and anything the demos revealed.
