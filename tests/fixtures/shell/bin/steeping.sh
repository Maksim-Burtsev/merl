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
#       ^ d: lib/infusion.sh:7
echo "$shout"

whisk 1 \
  | tr a b \
  | matcha
  # ^ d: lib/infusion.sh:11

whisk=$(whisk 2)
#       ^ d: lib/infusion.sh:7
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
# ^ d: lib/infusion.sh:7
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

notice="it's
  hot, isn't it"
ladle() { echo; }
ladle
# ^ d: bin/steeping.sh:91
echo it\'s
saucer() { echo; }
saucer
# ^ d: bin/steeping.sh:95
echo $'it\'s'
strainer() { echo; }
strainer
# ^ d: bin/steeping.sh:99

glaze() {
  coat="$1"
  if [ -n "$coat" ]; then
    coat="thin $coat"
    #           ^ d: bin/steeping.sh:104
  elif [ "$coat" = x ]; then
    coat="x $coat"
    #        ^ d: bin/steeping.sh:104
  else
    echo "$coat"
    #      ^ d: bin/steeping.sh:104
  fi
  echo "$coat"
  #      ^ d: bin/steeping.sh:109
}
rinse() {
  water=0
  if [ -n "$1" ]; then
    water=1
  fi
  if [ -n "$2" ]; then
    :
  else
    echo "$water"
    #      ^ d: bin/steeping.sh:121
  fi
}
drain() {
  dregs=0
  while read -r drip; do
    dregs="$drip"
  done
  if [ -n "$1" ]; then
    :
  else
    echo "$dregs"
    #      ^ d: bin/steeping.sh:133
  fi
}
sieve() {
  mesh="$1"
  case "$mesh" in
    fine)
      mesh="fine $mesh"
      ;;
    hot)
      mesh="hot"
      ;&
    warm)
      echo "$mesh"
      #      ^ d: bin/steeping.sh:149
      ;;
    coarse)
      echo "$mesh"
      #      ^ d: bin/steeping.sh:143
      case "$2" in
        a)
          mesh=a
          ;;
      esac
      echo "$mesh"
      #      ^ d: bin/steeping.sh:160
      ;;
  esac
  echo "$mesh"
  #      ^ d: bin/steeping.sh:160
}
simmer() {
  os="$1"
  os=$(case "$(uname)" in
    Darwin) echo mac ;;
    *) echo linux ;;
  esac)
  echo "$os"
  #      ^ d: bin/steeping.sh:172
}
decoct() {
  out=""
  while getopts "o:p" opt; do
    case "$opt" in
      o)
        out="$OPTARG"
        ;;
      p)
        echo "$out"
        #      ^ d: bin/steeping.sh:184
        ;;
    esac
  done
}
