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

SHIP_FLAGS = -O2
SHIP_FLAGS += -Wall
LABEL += boxed
ifndef TRACK
$(error usage: TRACK=1 make release)
endif
$(info labels: NOTE_TAG = $(NOTE_TAG))
NOTE_TAG += fragile

pack:: LABEL := sealed
$(PACK_FILES:.txt=.box): private override WRAP_MODE ?= gift
pack::
	@echo $(SHIP_FLAGS) $(LABEL) $(TRACK) $(NOTE_TAG) $(WRAP_MODE)
#        ^ d: mk/release.mk:14
#                      ^ d: picker mk/release.mk:16, mk/release.mk:23
#                               ^ d: none
#                                        ^ d: mk/release.mk:21
#                                                    ^ d: mk/release.mk:24
	SEAL+=wax; sh -c 'echo $$SEAL'
#                         ^ d: none
