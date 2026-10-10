#!/bin/sh
# Issue #241 demo: a real merl PR (#704) in two review rounds.
# Round 1 pushed d5d81ed0; round 2 added a fix (47e4cdba) and merged master (6a863787).
# usage: setup.sh SOURCE   (SOURCE: a clone of github.com/Maksim-Burtsev/merl holding those commits)
set -e
SRC=${1:-https://github.com/Maksim-Burtsev/merl.git}
rm -rf /tmp/241-origin.git /tmp/241-merl
git clone -q --bare --shared "$SRC" /tmp/241-origin.git 2>/dev/null || git clone -q --bare "$SRC" /tmp/241-origin.git
git -C /tmp/241-origin.git update-ref refs/heads/master bdd0a93c3ffaca5b7fb69bbee3f825dd5aff4c7d
git -C /tmp/241-origin.git update-ref refs/heads/b1004/undo-steps-478 d5d81ed03f049e89a1012a93c07b7484b8fc3899
git -C /tmp/241-origin.git symbolic-ref HEAD refs/heads/master
git clone -q /tmp/241-origin.git /tmp/241-merl
