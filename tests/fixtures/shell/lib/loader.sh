#!/bin/bash
# The project's own `load`, as bats-core's: outside a .bats file a `load` line is a call.
load() {
  source "$1.sh"
}

load warehouse
# ^ d: lib/loader.sh:3
