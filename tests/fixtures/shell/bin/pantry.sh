#!/usr/bin/env bash
# #1335: the file this script sources sources another, which defines skim: a picker.
source "$(dirname "$0")/../lib/rack.sh"

skim() {
  echo "pantry $1"
}

skim 1
# ^ d: picker bin/cauldron.sh:10, bin/pantry.sh:5, lib/hearth.sh:4
