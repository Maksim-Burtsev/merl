#!/usr/bin/env bash
# The shop of #307: every d case carries its answer in a comment under it.
source "$(dirname "$0")/../lib/pricing.sh"
. "$(dirname "$0")/../lib/warehouse.bash"

WEIGHT_LIMIT=30

gross() {
  discount "$(tariff_rate)"
  # ^ d: picker bin/basket.sh:35, lib/pricing.sh:22; want lib/pricing.sh:22 (#470)
  #           ^ d: lib/pricing.sh:14
}

bonus() {
  echo $(($(coupon_rate) + $(gross)))
  #          ^ d: picker lib/banner.sh:13, lib/pricing.sh:18; want lib/pricing.sh:18 (#436)
  #                          ^ d: bin/basket.sh:8
}

overweight() {
  local limit=$((WEIGHT_LIMIT + 20))
  #               ^ d: bin/basket.sh:6
  [ "$(weigh "$1")" -gt "$limit" ]
  #    ^ d: picker bin/basket.sh:29, lib/warehouse.bash:4; want lib/warehouse.bash:4 (#470)
  #                        ^ d: bin/basket.sh:21
}

hidden() {
  local weigh=$1
  echo "$weigh"
  #       ^ d: picker bin/basket.sh:29, lib/warehouse.bash:4; want bin/basket.sh:29 (#470)
}

restock() {
  local discount=$1
  echo $((discount + WEIGHT_LIMIT))
  #        ^ d: picker bin/basket.sh:35, lib/pricing.sh:22; want bin/basket.sh:35 (#470)
}

label() {
  describe
  # ^ d: picker lib/offers.sh:4, lib/pricing.sh:27
  echo "$CURRENCY$SYMBOL $RATE_CAP $LEDGER ${SURCHARGES[*]}"
  #       ^ d: lib/pricing.sh:7
  #               ^ d: lib/pricing.sh:8
  #                        ^ d: lib/pricing.sh:6
  #                                 ^ d: lib/pricing.sh:9
  #                                            ^ d: picker lib/pricing.sh:10, lib/pricing.sh:11
}

dispatch() {
  courier_dispatch "$COURIER_NAME"
  # ^ d: picker lib/banner.sh:6, lib/warehouse.bash:8; want lib/warehouse.bash:8 (#436)
  #                    ^ d: lib/warehouse.bash:2
  banner
  # ^ d: lib/banner.sh:4
}

tidy() {
  purge
  # ^ d: none; want bin/clean.sh:6 (#436)
  archive
  # ^ d: none; want bin/archive.sh:4 (#436)
  gross_all
  # ^ d: lib/pricing.sh:12
}
