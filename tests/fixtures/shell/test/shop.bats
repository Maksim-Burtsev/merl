#!/usr/bin/env bats
# #1058: a bats test file is Bash; `load` opens NAME.bash beside it.
load ../lib/warehouse
#           ^ d: lib/warehouse.bash:1
# ^ d: lib/loader.sh:3

@test "weighs a parcel" {
  parcel=12
  weigh "$parcel"
  # ^ d: lib/warehouse.bash:4
  #        ^ d: test/shop.bats:8
}

@test "another test's variable is not this test's" {
  echo "$parcel"
  #      ^ d: picker lib/warehouse.bash:12, test/shop.bats:8
}
