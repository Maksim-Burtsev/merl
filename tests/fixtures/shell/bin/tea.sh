#!/usr/bin/env bash
# #931: what bash's own binding makes hard for d.
source "$(dirname "$0")/../lib/tea.sh"

cat <<'EOF'
steep() {
EOF

cat <<-EOF
	kettle() {
	EOF

serve() {
  pour 1
# ^ d: lib/tea.sh:6
  echo "$pour ${pour} ${#pour} $((pour + 1))"
  #      ^ d: lib/tea.sh:4
  #             ^ d: lib/tea.sh:4
  #                      ^ d: lib/tea.sh:4
  #                               ^ d: lib/tea.sh:4
  echo '$pour'
  #      ^ d: none
  command steep
  #       ^ d: none
  brew
# ^ d: picker lib/tea.sh:21, lib/tea.sh:26
  case "$1" in
    pour) kettle ;;
#   ^ d: none
    #     ^ d: lib/tea.sh:16
    steep) steep ;;
#   ^ d: none
    #      ^ d: lib/tea.sh:10
  esac
}

for cup in 1 2; do
  echo "$cup"
  #      ^ d: bin/tea.sh:37
done

read -r teapot
echo "$teapot"
#      ^ d: bin/tea.sh:42

(( pour++ ))
#  ^ d: lib/tea.sh:4

strain
# ^ d: bin/infuse:9
