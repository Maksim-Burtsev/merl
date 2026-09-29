# Set only by appending or for one target, so d falls back to them (#499).
PACK_FLAGS += -z
WEIGHT_LIMIT += 5

release: PACK_VERSION := 1.0
release: gross
	@echo $(PACK_VERSION) $(PACK_FLAGS) $(WEIGHT_LIMIT)
#        ^ d: mk/release.mk:5
#                        ^ d: mk/release.mk:2
#                                      ^ d: Makefile:6
	STAMP+=1 true $(STAMP)
#                ^ d: none
