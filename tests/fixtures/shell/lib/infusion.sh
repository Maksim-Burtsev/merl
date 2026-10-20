#!/usr/bin/env bash
# #1126: the third corner, what bin/steeping.sh reaches.

leaf=sencha
date=monday

whisk() {
  echo "whisk $1"
}

matcha() {
  local leaf=$leaf
  #           ^ d: picker lib/infusion.sh:4; want lib/infusion.sh:4 (#1181)
  echo "$leaf"
  #      ^ d: lib/infusion.sh:12
}

if [[ -n ${TEA_FAST:-} ]]; then
  temper() { echo fast; }
else
  temper() { echo slow; }
fi

declare -A tins=([green]=1 [black]=2)

export -f whisk
#         ^ d: lib/infusion.sh:7
