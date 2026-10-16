#!/usr/bin/env bash
# #931: a function, a command and a variable of one name.

pour=2

pour() {
  echo "$1"
}

function steep {
  local pour=$1
  echo "$pour"
  #      ^ d: lib/tea.sh:11
}

kettle() (
  cd /tmp && pour 1
  #          ^ d: lib/tea.sh:6
)

brew () {
  steep 3
# ^ d: lib/tea.sh:10
}

brew() {
  steep 4
}
