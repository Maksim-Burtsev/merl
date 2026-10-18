#!/usr/bin/env bash
# #1057: a plain assignment binds its name in its function, else at the top level.

kettle_size=1

cat <<EOT
strength=fake
EOT

infuse_tea() {
  strength=mild
  strength=strong
  echo "$strength"
  #      ^ d: bin/brew.sh:12
  echo "${kettle_size}"
  #       ^ d: bin/brew.sh:4
  leaves=(green)
  leaves+=(black)
  echo "${leaves[*]}"
  #       ^ d: bin/brew.sh:17
}

kettle_size() {
  echo 2
}

pour_tea() {
  kettle_size
  # ^ d: bin/brew.sh:23
  echo "$strength"
  #      ^ d: picker bin/brew.sh:11, bin/brew.sh:12
}

# #1059: a sourced path opens its file, past a leading variable.
source "$TEA_HOME/lib/tea.sh"
#                  ^ d: lib/tea.sh:1
#         ^ d: none
. "${TEA_HOME}/lib/offers.sh"
#                   ^ d: lib/offers.sh:1
source "$(dirname "$0")/../lib/banner.sh"
#                              ^ d: lib/banner.sh:1
source "$TEA_HOME/lib/$flavour.sh"
#                       ^ d: none
