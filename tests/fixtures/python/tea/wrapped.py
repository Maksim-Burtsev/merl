from steeplab import (
    fixture,  # the package's own, never the one of rigs/helpers.py
    leaf as steeper,
)


# #1452: an import wrapped in `( … )` over several lines is read for the line of its name, so
# `steeplab`, neither in the project nor on disk, answers that line, never a namesake of
# `rigs/helpers.py`, which this file does not import.
@fixture
#^ d: tea/wrapped.py:2
# status: via import steeplab (not installed)
def pour():
    return steeper()
#          ^ d: tea/wrapped.py:3
