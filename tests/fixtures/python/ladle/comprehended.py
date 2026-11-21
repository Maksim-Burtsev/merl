from typing import List

from ladle.cup import Cup
from ladle.jar import Jar
from ladle.mug import Mug


def go(cups: List[Cup], mugs: list[Mug], loose):
    x = Jar()
    [x.fill() for x in cups]
    #  ^ d: ladle/cup.py:2
    {x.size() for x in mugs if x}
    #  ^ d: ladle/mug.py:5
    sum(x.size() for x in cups)
    #     ^ d: ladle/cup.py:5
    [t.fill() for t in loose]
    #  ^ d: !jump
    [x.fill() for x in cups] + [x.size() for x in mugs]
    #                             ^ d: !jump
    list(map(lambda x: x.fill(), [x for x in cups]))
    #                    ^ d: !jump
    [x.fill() for x in cups if x.size()]
    #                            ^ d: ladle/cup.py:5
    x.fill()
    # ^ d: ladle/jar.py:2
