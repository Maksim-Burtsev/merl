#!/bin/bash
BIN=$1; merl() { "$BIN" "$@"; }; while printf '$ ' && read -r line; do eval "$line"; done
