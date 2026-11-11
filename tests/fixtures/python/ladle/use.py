from typing import Callable

from ladle.cup import Cup
from ladle.mug import Mug


def go(cups: list[Cup], loose):
    list(map(lambda c: c.fill(), cups))
    #                    ^ d: !jump; want ladle/cup.py:2 (#1488)
    sorted(cups, key=lambda c: c.size())
    #                            ^ d: !jump; want ladle/cup.py:5 (#1488)
    f: Callable[[Cup], bool] = lambda x: x.fill()
    #                                      ^ d: !jump; want ladle/cup.py:2 (#1488)
    c = Mug()
    list(map(lambda c: c.fill(), cups))
    #                    ^ d: !jump; want ladle/cup.py:2 (#1488)
    c.fill()
    # ^ d: ladle/mug.py:2
    list(map(lambda t: t.fill(), loose))
    #                    ^ d: !jump
