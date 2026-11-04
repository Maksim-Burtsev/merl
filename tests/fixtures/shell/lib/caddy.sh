#!/usr/bin/env bash
# #1335: a function run only after `_caddy_open &&` reads what _caddy_open keeps when it succeeds.
caddy_add() {
  _caddy_open && _caddy_fill && _caddy_seal
}

caddy_rm() {
  _caddy_open && _caddy_fill
  _caddy_close
  ! _caddy_open && _caddy_peek
  _caddy_warm && _caddy_pour
}

_caddy_open() {
  CADDY_LEAF="${CADDY_LEAF:-sencha}"
  CADDY_LID="${CADDY_LID:-tight}"
  if [ -z "$CADDY_LEAF" ] || [ -z "$CADDY_LID" ]; then
    CADDY_LEAF=""
    CADDY_LID=""
    echo "the caddy's empty" >&2
    return 1
  fi
}

_caddy_fill() {
  echo "fill $CADDY_LEAF"
  #           ^ d: lib/caddy.sh:15
}

_caddy_seal() {
  echo "seal $CADDY_LID"
  #           ^ d: picker lib/caddy.sh:16, lib/caddy.sh:19
}

_caddy_close() {
  echo "close $CADDY_LID"
  #            ^ d: picker lib/caddy.sh:16, lib/caddy.sh:19
}

_caddy_peek() {
  echo "peek $CADDY_LEAF"
  #           ^ d: picker lib/caddy.sh:15, lib/caddy.sh:18
}

_caddy_warm() {
  if [ -n "$CADDY_LEAF" ]; then
    CADDY_HEAT=hot
  else
    CADDY_HEAT=""
    return 1
  fi
}

_caddy_pour() {
  echo "pour $CADDY_HEAT"
  #           ^ d: picker lib/caddy.sh:47, lib/caddy.sh:49
}
