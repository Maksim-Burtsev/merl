from steeplab import leaf, \
    fixture, \
    steeper


# #1452: an import continued with a backslash binds every name on its lines, so `steeplab`,
# neither in the project nor on disk, answers the line of the name, never a namesake of
# `rigs/helpers.py`, which this file does not import.
@fixture
#^ d: tea/continued.py:2
# status: via import steeplab (not installed)
def brew():
    return steeper(leaf)
#          ^ d: tea/continued.py:3
#                  ^ d: tea/continued.py:1


# A comment that ends in a backslash continues nothing: the import under it stands alone.
import os  # cache dir is C:\
from steepkit import pourer


def pour():
    return pourer(os)
#          ^ d: tea/continued.py:20


import steepcan, \
    steepjar


def jar():
    return steepjar.lid(steepcan)
#          ^ d: tea/continued.py:29
#                       ^ d: tea/continued.py:28
