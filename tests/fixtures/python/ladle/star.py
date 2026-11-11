from ladle.cup import Cup
from ladle.helpers import *


def go(cups: list[Cup]):
    max(cups, key=lambda c: c.size())
    #                         ^ d: !jump
