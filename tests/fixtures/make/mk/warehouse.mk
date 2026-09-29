GRAMS ?= 1500

weigh:
	@echo $$(( $(GRAMS) / 1000 ))

describe:
	@echo courier
