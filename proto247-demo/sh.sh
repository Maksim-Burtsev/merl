#!/bin/bash
BIN=$1
merl() { "$BIN" "$@"; }
while printf '\033[1;34m%s\033[0m $ ' "${PWD##*/}" && read -r line; do eval "$line"; done
