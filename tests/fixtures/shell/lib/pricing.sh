#!/usr/bin/env bash
# The shop of #307: every d case carries its answer in a comment under it.
# A comment that reads like code declares nothing:
# weigh() {

export RATE_CAP=100
declare -r CURRENCY="EUR"
readonly SYMBOL="E"
typeset -i LEDGER=0
SURCHARGES=()
SURCHARGES+=(post)
alias gross_all='gross'

tariff_rate() {
  echo 1
}

function coupon_rate {
  echo 2
}

function discount() {
  local total=$1
  echo $((total < RATE_CAP ? total - 1 : RATE_CAP - 1))
}

describe() { echo "tariff$SYMBOL"; }
