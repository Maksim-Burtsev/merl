from typing import Callable

from ladle.cup import Cup
from ladle.mug import Mug


def go(cups: list[Cup], loose):
    list(map(lambda c: c.fill(), cups))
    #                    ^ d: ladle/cup.py:2
    sorted(cups, key=lambda c: c.size())
    #                            ^ d: ladle/cup.py:5
    f: Callable[[Cup], bool] = lambda x: x.fill()
    #                                      ^ d: ladle/cup.py:2
    c = Mug()
    list(map(lambda c: c.fill(), cups))
    #                    ^ d: ladle/cup.py:2
    c.fill()
    # ^ d: ladle/mug.py:2
    list(map(lambda t: t.fill(), loose))
    #                    ^ d: !jump
    max(cups, key=lambda c: c.size())
    #                         ^ d: ladle/cup.py:5
    list(map(lambda c, d: c.fill(), cups, loose))
    #                       ^ d: !jump


def mugs(m: list[Mug]):
    m.sort(key=lambda c: c.size())
    #                      ^ d: ladle/mug.py:5
