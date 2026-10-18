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
