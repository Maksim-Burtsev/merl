#!/usr/bin/env bash
COURIER_NAME=${COURIER_NAME:-post}

weigh() {
  echo $(($1 / 1000))
}

courier_dispatch() {
  echo "$COURIER_NAME: $1"
}
