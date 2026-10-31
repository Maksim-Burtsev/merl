#!/usr/bin/env bash
# #1335: a command this script defines once, sourcing nothing, is its own; `env` hands a name to a program.

stew() {
  echo "samovar $1"
}

boil() {
  stew 1
  # ^ d: bin/samovar.sh:4
}

env -i SAMOVAR_HEAT=90 SAMOVAR_TEA=puer bash bin/glass.sh
echo "$SAMOVAR_TEA"
#      ^ d: none
