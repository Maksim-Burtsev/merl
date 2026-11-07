#!/usr/bin/env bash
# #1426: a "…" or $'…' string over lines is text, as the shell reads it.
source "$(dirname "$0")/../lib/infusion.sh"

menu="
whisk() {
kombucha() {
KETTLE=1
"
echo "brewing
puerh() {
"
printf '%s\n' 'a
"
'
w="$(printf 'x')
assam() {
"
z="it's \"cold\"
rooibos() {
"
echo $'it\'s
genmaicha() {
'
echo \"
yerba() { :; }

whisk 1
# ^ d: lib/infusion.sh:7
kombucha
# ^ d: none
echo "$KETTLE"
#      ^ d: none
puerh
# ^ d: none
assam
# ^ d: none
rooibos
# ^ d: none
genmaicha
# ^ d: none
yerba
# ^ d: bin/quoted.sh:26
