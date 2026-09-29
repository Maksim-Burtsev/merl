#!/usr/bin/env bash
# What a heredoc and a quoted string hold is text, not code.

banner() {
  cat <<EOT
courier_dispatch() {
  echo fake
}
EOT
}

usage='
coupon_rate() {
'
