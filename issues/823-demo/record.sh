#!/bin/bash
# record.sh BIN MODE STEPS OUT [ROUND2]: round 1 on BIN, the author's push, then round 2 recorded.
set -e
BIN=$1; MODE=$2; STEPS=$3; OUT=$4; R2=${5:-${DEMO:-.}/round2.sh}
CAST=${CAST:-tools/cast.py}
${DEMO:-.}/setup.sh
python3 $CAST --bin $BIN --keys ${DEMO:-.}/round1.steps --project /tmp/823/parcel --size 160x50 -o ${DEMO:-.}/round1-throwaway.gif -- --review >/dev/null
$R2 >/dev/null
python3 $CAST --bin ${DEMO:-.}/prompt.sh --keys $STEPS --project /tmp/823/parcel --size 160x50 -o $OUT ${GHOSTTY:+--ghostty "$GHOSTTY"} -- $BIN $MODE
