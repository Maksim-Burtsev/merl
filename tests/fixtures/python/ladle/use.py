from typing import Callable

from ladle.cup import Cup
from ladle.jar import Jar
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
    list(map(lambda c, d: c.fill(), cups))
    #                       ^ d: !jump


def mugs(m: list[Mug]):
    m.sort(key=lambda c: c.size())
    #                      ^ d: ladle/mug.py:5


def rebound(cups: list[Cup], jars: list[Jar]):
    list(map(lambda c: [c.fill() for c in jars], cups))
    #                     ^ d: !jump
    sorted(cups, key=lambda c: sum(c.size() for c in jars))
    #                                ^ d: !jump
    list(map(lambda c: (c := jars[0]) and c.fill(), cups))
    #                                       ^ d: !jump
