#!/bin/bash
B=$(cd "$(dirname "$0")" && pwd); P=$B/proj; cd $B
for spec in "python paperless-ngx" "ts outline" "js eslint" "go caddy" "rust ripgrep" "c redis" "cpp leveldb" "php koel" "swift Alamofire"; do
  set -- $spec
  if [ -z "$ONLY" ] || [ "$ONLY" = "$1" ]; then
    nohup python3 bench.py oracle $1 $P/$2 cur-$1.tsv orc-$1.tsv > log-orc-$1.txt 2>&1 &
  fi
done
