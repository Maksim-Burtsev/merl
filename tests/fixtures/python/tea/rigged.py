import steeplab


# #1452: `steeplab` is neither in the project nor on disk, so its names are the import's, never
# the `fixture` of `rigs/helpers.py`, which this file does not import.
@steeplab.fixture
#         ^ d: tea/rigged.py:1
#         status: via import steeplab (not installed)
def kettle_on():
    return 1


hook = steeplab.fixture
#               ^ d: tea/rigged.py:1
