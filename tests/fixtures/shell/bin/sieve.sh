#!/usr/bin/env bash
# #1335: a shellcheck source= directive names the file a variable path reads: its own stir.
# shellcheck source=lib/spoons.sh
source "$SHELF/spoons.sh"

stir() {
  echo sieve
}

stir
# ^ d: bin/sieve.sh:6
