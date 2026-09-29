#!/usr/bin/env bash
# #470: a quoted value, an array, a `declare -p` and a one-line function declare no local.

notify() {
  local msg="restock the gross shelf" count=(a b c)
  gross
  # ^ d: bin/basket.sh:8
  echo "$msg ${count[*]}"
  #            ^ d: bin/notify.sh:5
  declare -p WEIGHT_LIMIT
  echo "$WEIGHT_LIMIT"
  #      ^ d: bin/basket.sh:6
}

die() { echo "$1"; exit 1; }  # abort
declare -A PRICES=([a]=1)
build() {
  echo "${PRICES[a]}"
  #       ^ d: bin/notify.sh:16
}
