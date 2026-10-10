#!/bin/bash
# A shell prompt for the demo: `merl` is the binary under test; the narration line
# "# the author pushes ..." moves origin's branch to round 2 (ROUND, a script).
BIN=$1; ROUND=$2; [ -n "$3" ] && export MERL_P241=$3
merl() { "$BIN" "$@"; }
while printf '$ ' && read -r line; do
  case $line in "# the author pushes"*) "$ROUND" >/dev/null 2>&1;; esac
  eval "$line"
done
