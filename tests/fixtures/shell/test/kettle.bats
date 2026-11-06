#!/usr/bin/env bats
# #1335: `run` runs a command, so its first word is one; this file defines `main` once.

main() {
  echo kettle
}

@test "runs main" {
  run main
  #   ^ d: test/kettle.bats:4
  run -1 --separate-stderr main
  #                        ^ d: test/kettle.bats:4
  run ! main
  #     ^ d: test/kettle.bats:4
}
