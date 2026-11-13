#!/bin/bash
S=$(cd "$(dirname "$0")" && pwd)
W=$(cd "$S/.." && pwd)
out=$1 steps=$2 bin=$3 size=$4 scene=$5 done_n=$6 gone=$7; shift 7
[ "$gone" = - ] && gone=
D=$("$S/scene.sh" "$scene" "$done_n" "$gone") || { echo "FAIL scene $scene"; exit 1; }
python3 "$W/tools/cast.py" --bin "$S/sh.sh" --keys "$S/steps/$steps.steps" --project "$D" --size "$size" -o "$out" "$@" -- "$bin" && echo "OK $out" || echo "FAIL $out"
