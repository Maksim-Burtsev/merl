#!/bin/sh
# prep.sh BIN [P1521]: round 2 of #1307's demo, CHANGELOG.md ticked with m, then the agent adds a second answer on its CHANGELOG thread.
set -e
./reset.sh notes-round2-A.txt >/dev/null 2>&1
tmux -L p1521prep kill-server 2>/dev/null || true
tmux -L p1521prep new-session -d -s m -x 160 -y 50 "env MERL_BIN=$1 ${2:+MERL_P1521=$2} ./run.sh --review"
sleep 2
tmux -L p1521prep send-keys -t m Tab Down Down Down Down Down Down Down Down Down
sleep 0.5
[ "$2" = B ] || tmux -L p1521prep send-keys -t m m
sleep 0.7
tmux -L p1521prep send-keys -t m q
sleep 1
tmux -L p1521prep kill-server 2>/dev/null || true
cd /tmp/1521-httpx && python3 -c "
p='.git/merl/notes/agent/unquote'; s=open(p).read()
open(p,'a').write('agent: moved the line under Fixed, where the other Digest entries are\n')
"
