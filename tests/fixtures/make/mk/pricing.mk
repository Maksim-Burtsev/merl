TARIFF_RATE = 1
COUPON_RATE := 2
RATE_CAP = 100

define discount
$(shell expr $(1) - 1)
endef

tariff-rate coupon-rate: ; @echo $(RATE_CAP)
#                                   ^ d: mk/pricing.mk:3

describe:
	@echo tariff
