#!/usr/bin/env bash
# #1335: what bin/samovar.sh hands over by `env`, and a namesake defined before a `source`.

echo "$SAMOVAR_HEAT $SAMOVAR_TEA"
#      ^ d: bin/samovar.sh:13
#                    ^ d: bin/samovar.sh:13

stew() {
  echo "glass $1"
}
source "$(dirname "$0")/../lib/stove.sh"
stew 1
# ^ d: picker bin/glass.sh:8, bin/samovar.sh:4, lib/stove.sh:4
