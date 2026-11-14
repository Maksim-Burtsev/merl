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


# The review of #1471: a name the file binds again for when the package is missing is that
# binding's, as on master.
try:
    from steepfast import steep_loads
except ImportError:

    def steep_loads(s):
        return s


try:
    from steepyaml import steeper
except ImportError:
    from tea.rigs.helpers import steeper


loaded = steep_loads("x")
#        ^ d: tea/rigged.py:23
steeped = steeper()
#         ^ d: tea/rigs/helpers.py:5
