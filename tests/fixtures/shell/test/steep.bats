#!/usr/bin/env bats
# #1335: `split_into` in lib/steep.bash fills `cups` through its argument.

load ../lib/steep

@test "decants" {
  decant tea
  [ "${cups[0]}" = tea ]
  #    ^ d: lib/steep.bash:17
  [ "$whistle" = "" ]
  #    ^ d: none
  [ "$sediment" = "" ]
  #    ^ d: none
}
