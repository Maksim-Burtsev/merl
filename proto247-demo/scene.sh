#!/bin/bash
set -e
S=$(cd "$(dirname "$0")" && pwd)
scene=$1; done_n=${2:-none}; gone=${3:-}
D=/tmp/247r7/$scene
python3 "$S/mkfix.py" "$D" >/dev/null
cd "$D/services"
printf 'api:ACME-56\nweb:ACME-56\ndocs:ACME-56\nweb:ACME-91\nbot:ACME-11\n' > today.txt
printf 'web:ACME-91\nbot:ACME-11\napi:ACME-102\ndocs:ACME-105\n' > tomorrow.txt
printf 'api:ACME-56\nweb:ACME-65\n' > typo.txt
mkdir -p "$D/elsewhere"
if [ "$done_n" != none ]; then
  n=0
  while IFS=: read -r repo branch; do
    n=$((n+1))
    if [ "$n" -le "$done_n" ]; then st=done; else st=left; fi
    printf '%s\t%s\t%s\n' "$repo" "$branch" "$st"
  done < today.txt > .merl-batch
fi
if [ -n "$gone" ]; then
  git -C "$D/origin/${gone%%:*}.git" branch -q -D "${gone##*:}"
fi
echo "$D/services"
