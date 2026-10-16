#!/bin/sh
BIN=$1; shift
for s in $SENDS; do
  ( sleep "${s%%@*}"; "$BIN" --send "${s#*@}" ) &
done
exec "$BIN" "$@"
