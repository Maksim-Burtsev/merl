#!/usr/bin/env bash
# #1126: the third corner, its answers read from the code.
source "$(dirname "$0")/../lib/infusion.sh"

: '
whisk() {
'

cat <<'EOF'
matcha() {
EOF

date +%s
# ^ d: none
echo "$date"
#      ^ d: lib/infusion.sh:5

shout=$(whisk 1 | tr a-z A-Z)
#       ^ d: picker bin/steeping.sh:6, lib/infusion.sh:7; want lib/infusion.sh:7 (#1180)
echo "$shout"

whisk 1 \
  | tr a b \
  | matcha
  # ^ d: lib/infusion.sh:11

whisk=$(whisk 2)
#       ^ d: picker bin/steeping.sh:6, lib/infusion.sh:7; want lib/infusion.sh:7 (#1180)
echo "$whisk"
#      ^ d: bin/steeping.sh:27

for matcha in a b; do
  matcha "$matcha"
  # ^ d: lib/infusion.sh:11
  #        ^ d: bin/steeping.sh:32
done

while IFS=: read -r grade tin; do
  echo "$tin $grade"
  #      ^ d: bin/steeping.sh:38
  #           ^ d: bin/steeping.sh:38
done < /dev/null

read -ra cups <<< "a b"
echo "${cups[0]}"
#       ^ d: bin/steeping.sh:44

mapfile -t lines < /dev/null
echo "${lines[@]}"
#       ^ d: bin/steeping.sh:48

printf -v stamp '%s' now
echo "$stamp"
#      ^ d: bin/steeping.sh:52

declare -n ref=leaf
#              ^ d: lib/infusion.sh:4
echo "$ref ${tins[green]} ${leaf:-none} ${#tins[@]}"
#      ^ d: bin/steeping.sh:56
#            ^ d: lib/infusion.sh:24
#                           ^ d: lib/infusion.sh:4
#                                          ^ d: lib/infusion.sh:24

temper
# ^ d: picker lib/infusion.sh:19, lib/infusion.sh:21

cat <<EOT
$(whisk 3) and $leaf
# ^ d: picker bin/steeping.sh:6, lib/infusion.sh:7; want lib/infusion.sh:7 (#1180)
#               ^ d: lib/infusion.sh:4
EOT

cleanup_cups() {
  rm -f /tmp/cups
}
trap 'cleanup_cups' EXIT
#      ^ d: bin/steeping.sh:73

outer_tray() {
  inner_tray() { echo inner; }
  inner_tray
  # ^ d: bin/steeping.sh:80
}

[[ -v leaf ]] && unset leaf
#     ^ d: lib/infusion.sh:4
#                      ^ d: lib/infusion.sh:4
