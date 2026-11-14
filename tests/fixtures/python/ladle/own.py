from ladle.cup import Cup
from ladle.helpers import sorted


def map(f, xs):
    return xs


def go(cups: list[Cup]):
    list(map(lambda c: c.fill(), cups))
    #                    ^ d: !jump
    sorted(cups, key=lambda c: c.size())
    #                            ^ d: !jump
