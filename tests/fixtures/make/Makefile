# The shop of #307: every d case carries its answer in a comment under it.
# A comment that reads like a rule declares nothing:
# weigh: gross
include mk/pricing.mk mk/warehouse.mk

WEIGHT_LIMIT = 30
export COURIER ?= post
override CURRENCY := EUR
SHELL != command -v bash

.PHONY: all gross bonus weigh-parcel dispatch

all: gross bonus dispatch
#    ^ d: Makefile:18
#          ^ d: Makefile:24
#                ^ d: picker Makefile:37, Makefile:43

gross: tariff-rate
#      ^ d: mk/pricing.mk:9
	@echo $(call discount,$(TARIFF_RATE))
#                  ^ d: none; want mk/pricing.mk:5 (#468)
#                              ^ d: mk/pricing.mk:1

bonus: coupon-rate gross
#      ^ d: mk/pricing.mk:9
#                  ^ d: Makefile:18
	@echo $(COUPON_RATE) $(CURRENCY)
#          ^ d: mk/pricing.mk:2
#                         ^ d: Makefile:8

weigh-parcel: weigh
#             ^ d: mk/warehouse.mk:3
	@test $(GRAMS) -lt $(WEIGHT_LIMIT)
#          ^ d: mk/warehouse.mk:1
#                        ^ d: Makefile:6

dispatch:: weigh-parcel
#          ^ d: Makefile:31
	@echo "$(COURIER) via $(SHELL)"
#           ^ d: Makefile:7
#                         ^ d: Makefile:9

dispatch:: describe
#          ^ d: picker mk/pricing.mk:12, mk/warehouse.mk:6

tidy: purge archive
#     ^ d: none; want mk/clean.mk:4 (#436)
#           ^ d: none; want mk/archive.mk:2 (#436)

include mk/clean.mk mk/archive.mk
