#!/bin/bash
# usage: MASTER_MERL=... proto-925/record.sh OUTDIR [extra cast args...]; ONLY="jump review" picks GIFs
set -u
S=$(cd "$(dirname "$0")" && pwd); P=$(dirname "$S")
M=${MASTER_MERL:?a release build of master 7253b460}; X=$P/target/release/merl
O=$1; shift; mkdir -p $O
F=/tmp/925-fastapi
if [ ! -d $F ]; then
  git clone -q https://github.com/fastapi/fastapi.git $F && git -C $F checkout -q 28a206107302ee20ce6a9a876d05a258c1c8d328
  git -C $F checkout -q -b fix/response-errors && git -C $F am -q $S/review.patch
fi
reset() { rm -rf $F/.git/merl; git -C $F checkout -q -- .; git -C $F checkout -q fix/response-errors; tmux -L r925 kill-server 2>/dev/null; sleep 0.3; }
cast() { (cd $P && tools/heavy tools/cast.py --size 160x50 --project $F "$@" 2>&1 | tail -1); }
only=${ONLY:-now jump review picker edit pastend unchanged shell-a shell-b shell-c light}
for g in $only; do
  reset
  case $g in
    now) cast --bin $M --keys $S/now.steps -o $O/now.gif "$@" -- fastapi/routing.py:384 ;;
    jump) SENDS="5.5@fastapi/dependencies/utils.py:586" cast --bin $S/recv.sh --keys $S/jump.steps -o $O/jump.gif "$@" -- $X fastapi/routing.py:384 ;;
    light) SENDS="5.5@fastapi/dependencies/utils.py:586" cast --bin $S/recv.sh --keys $S/jump.steps -o $O/light.gif --ghostty 'TokyoNight Day' "$@" -- $X -t tokyonight-day fastapi/routing.py:384 ;;
    review) SENDS="5.5@fastapi/dependencies/utils.py:586" cast --bin $S/recv.sh --keys $S/review.steps -o $O/review.gif "$@" -- $X --review ;;
    picker) SENDS="5@fastapi/dependencies/utils.py:586" cast --bin $S/recv.sh --keys $S/picker.steps -o $O/picker.gif "$@" -- $X fastapi/routing.py:384 ;;
    edit) SENDS="5.5@fastapi/dependencies/utils.py:586" cast --bin $S/recv.sh --keys $S/edit.steps -o $O/edit.gif "$@" -- $X fastapi/routing.py:384 ;;
    pastend) SENDS="3@fastapi/encoders.py:9999" cast --bin $S/recv.sh --keys $S/pastend.steps -o $O/pastend.gif "$@" -- $X fastapi/routing.py:384 ;;
    unchanged) cast --bin $M --keys $S/unchanged.steps -o $O/unchanged-master.gif "$@" -- fastapi/routing.py:384
               reset; cast --bin $X --keys $S/unchanged.steps -o $O/unchanged-a.gif "$@" -- fastapi/routing.py:384 ;;
    shell-a) tmux -L r925 -f /dev/null new-session -d -x 160 -y 50 -c $F "$X fastapi/routing.py"; sleep 1.5
             ( sleep ${KILL_AT:-12}; tmux -L r925 kill-server; ) &
             cast --bin $S/prompt.sh --keys $S/shell-a.steps -o $O/shell-a.gif --type-delay 0.03 --size 160x10 "$@" -- $X ;;
    shell-b) tmux -L r925 -f /dev/null new-session -d -x 160 -y 50 -c $F "env MERL_925=B $X fastapi/routing.py"; sleep 1.5
             MERL_925=B cast --bin $S/prompt.sh --keys $S/shell-b.steps -o $O/shell-b.gif --type-delay 0.03 --size 160x10 "$@" -- $X ;;
    shell-c) tmux -L r925 -f /dev/null new-session -d -x 160 -y 50 -c $F "env MERL_925=C $X --listen main fastapi/routing.py"; sleep 1.5
             MERL_925=C cast --bin $S/prompt.sh --keys $S/shell-c.steps -o $O/shell-c.gif --type-delay 0.03 --size 160x10 "$@" -- $X ;;
  esac
done
reset
