#!/bin/sh
# SessionStart hook (#542). Claude Code loads AGENTS.md from the checkout a session starts in,
# and the main checkout's master is updated by nobody: all work happens in worktrees. This
# fast-forwards a master that has nothing to lose, and tells the agent when the AGENTS.md it
# loaded is older than origin's. What it prints goes into the session's context; it prints
# nothing when the rules are current, and never fails a session (offline, no origin, local work).
cd "${CLAUDE_PROJECT_DIR:-.}" 2>/dev/null || exit 0
git rev-parse --git-dir >/dev/null 2>&1 || exit 0
GIT_TERMINAL_PROMPT=0 git fetch -q origin master 2>/dev/null
git rev-parse -q --verify origin/master >/dev/null || exit 0

base=$(git merge-base HEAD origin/master 2>/dev/null) || exit 0
old=$(git rev-parse -q --verify "$base:AGENTS.md")
new=$(git rev-parse -q --verify origin/master:AGENTS.md)
behind=$(git rev-list --count HEAD..origin/master)

pulled=
if [ "$behind" != 0 ] && [ "$(git symbolic-ref -q --short HEAD)" = master ] &&
    git merge -q --ff-only origin/master >/dev/null 2>&1; then
    pulled=1
fi

[ "$old" = "$new" ] && exit 0
if [ -n "$pulled" ]; then
    echo "This checkout's master was $behind commits behind origin and is now fast-forwarded." \
        "AGENTS.md changed on the way: read it again before you act on it."
else
    echo "The AGENTS.md loaded into this session is older than origin's ($behind commits behind)." \
        "Read the current one before you act on it: git show origin/master:AGENTS.md"
fi
