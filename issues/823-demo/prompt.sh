#!/bin/bash
# A shell prompt for the demo: `merl` is the binary under test, $2 picks the prototype's option (A, B, C).
BIN=$1; [ -n "$2" ] && export MERL_P823=$2
merl() { "$BIN" "$@"; }
while printf '$ ' && read -r line; do eval "$line"; done
