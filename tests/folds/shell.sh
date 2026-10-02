#!/usr/bin/env bash
set -euo pipefail

# if this were code it would fi

usage() {  # f: 6-11
  cat <<EOF  # f: 7-10
usage: $0 [if|case|done]  # f: 6-11
  fi esac  # f: 6-11
EOF
}  # f: 6-11

function build {  # f: 13-37
  local target=${1:-all}  # f: 13-37
  if [[ "$target" == "all" ]]; then  # f: 15-22
    make all  # f: 13-37
  elif [ "$target" = "fi" ]; then  # f: 17-18
    echo 'if then fi'  # f: 13-37
  else  # f: 19-21
    echo "case $target in"  # f: 13-37
    echo "done"  # f: 13-37
  fi  # f: 13-37
  for f in *.c; do  # f: 23-25
    cc -c "$f"  # f: 13-37
  done  # f: 13-37
  while read -r line; do  # f: 26-28
    echo "$line # done"  # f: 13-37
  done < input.txt  # f: 13-37
  case "$target" in  # f: 29-36
    if|fi)  # f: 13-37
      echo "keyword"  # f: 13-37
      ;;  # f: 13-37
    *)  # f: 13-37
      echo "other"  # f: 13-37
      ;;  # f: 13-37
  esac  # f: 13-37
}  # f: 13-37

clean()  # f: 39-50
{  # f: 39-50
  rm -rf build  # f: 39-50
  until [ -z "$(ls)" ]; do  # f: 42-44
    sleep 1  # f: 39-50
  done  # f: 39-50
  cat <<-'DONE'  # f: 45-47
	if fi  # f: 39-50
	DONE
  x=$'it\'s'  # f: 39-50
  echo "${x#*if}" $#  # f: 39-50
}  # f: 39-50

main() {  # f: 52-58
  usage  # f: 52-58
  build "$@"  # f: 52-58
  for ((i = 0; i < 3; i++)); do  # f: 55-57
    echo "$i"  # f: 52-58
  done  # f: 52-58
}  # f: 52-58

tidy() {  # f: 60-69
  for f in *; do  # f: 61-65
    case "$f" in  # f: 62-64
      done) rm "$f" ;;  # f: 60-69
    esac  # f: 60-69
  done  # f: 60-69
  read -r a <<< "$f"  # f: 60-69
  n="$(grep -c '"' "$a")"  # f: 60-69
  echo "$n"  # f: 60-69
}  # f: 60-69

main "$@"
