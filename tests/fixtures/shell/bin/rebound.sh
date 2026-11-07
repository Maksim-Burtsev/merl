#!/usr/bin/env bash
mug=big
chai() {
  echo chai
}
two() { local mug=small; echo "$mug"; }
#                               ^ d: bin/rebound.sh:6
for mug in a b; do echo "$mug"; done
#                         ^ d: bin/rebound.sh:8
while read -r mug; do echo "$mug"; done < /dev/null
#                            ^ d: bin/rebound.sh:10
mug=x; echo "$mug"
#             ^ d: bin/rebound.sh:12
( mug=sub; echo "$mug" )
#                 ^ d: bin/rebound.sh:14
case $1 in a) mug=1; echo "$mug";; esac
#                           ^ d: bin/rebound.sh:16
mapfile -t mug < /dev/null; echo "${mug[0]}"
#                                   ^ d: bin/rebound.sh:18
three() { local x=1 mug=13; echo "$mug"; }
#                                  ^ d: bin/rebound.sh:20
mug=y
mug=z echo "$mug"
#            ^ d: bin/rebound.sh:22
echo "$mug" "$mug"
#      ^ d: bin/rebound.sh:22
four() { local mug=1; }; echo "$mug"
#                               ^ d: bin/rebound.sh:22
# local mug=2; echo "$mug"
echo "$mug"
#      ^ d: bin/rebound.sh:22
echo "local mug=3"; echo "$mug"
#                          ^ d: bin/rebound.sh:22
echo 'mug=4'; echo "$mug"
#                    ^ d: bin/rebound.sh:22
( mug=5 ); echo "$mug"
#                 ^ d: bin/rebound.sh:22
mug=$(echo "$mug")
#            ^ d: bin/rebound.sh:22
five() { local mug=12 tea=$mug; }
#                          ^ d: bin/rebound.sh:38
six() { echo "$mug"; local mug=6; }
#              ^ d: bin/rebound.sh:38
seven() {
  local chai=1
  echo "$chai"
#        ^ d: bin/rebound.sh:45
  chai
# ^ d: bin/rebound.sh:3
}
eight() {
  mug=`date +%s`
  echo "$mug"
#        ^ d: bin/rebound.sh:52
  mug=${HOME:- x}
  echo "$mug"
#        ^ d: bin/rebound.sh:55
  mug=a\ b
  echo "$mug"
#        ^ d: bin/rebound.sh:58
  mug=${mug// /_}
  echo "$mug"
#        ^ d: bin/rebound.sh:61
  mug=r 2>/dev/null
  echo "$mug"
#        ^ d: bin/rebound.sh:64
  mug=s > /dev/null; echo "$mug"
#                           ^ d: bin/rebound.sh:67
  mug=$(date +%s)
  echo "$mug"
#        ^ d: bin/rebound.sh:69
}
nine() {
  local mug=0
  mug=$((mug + 1))
# ^ d: bin/rebound.sh:74
  mug+=1
# ^ d: bin/rebound.sh:74
  [ -z "$mug" ] && mug=5
#                  ^ d: bin/rebound.sh:74
  if true; then mug=7; fi
#               ^ d: bin/rebound.sh:74
  echo "$mug"
#        ^ d: bin/rebound.sh:74
}
