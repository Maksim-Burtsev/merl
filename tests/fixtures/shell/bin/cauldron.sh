#!/usr/bin/env bash
# #1335: a script whose one sourced file holds no namesake and sources nothing calls its own skim.
source "$(dirname "$0")/../lib/spoons.sh"

skimming() {
  skim 1
  # ^ d: bin/cauldron.sh:10
}

skim() {
  echo "cauldron $1"
}
