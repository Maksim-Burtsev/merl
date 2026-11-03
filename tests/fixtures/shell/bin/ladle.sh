#!/usr/bin/env bash
# #1335: a sourced path that starts from a variable may name any file, so stir stays a picker.
source "$SHELF/spoons.sh"

stir() {
  echo ladle
}

stir
# ^ d: picker bin/ladle.sh:5, bin/sieve.sh:6
