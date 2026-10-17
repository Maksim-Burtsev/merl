from __future__ import annotations

import functools
from typing import TYPE_CHECKING

from tea.kettle import Infusion, Kettle, Teapot, steep

if TYPE_CHECKING:
    from tea.stale import Kettle as StaleKettle

try:
    from tea.fast import chill
except ImportError:
    from tea.stale import steep as chill

brews = 0
cup = 1


def local_steep() -> int:
    steep = 3
    return steep
    #      ^ d: tea/brewing.py:21


def imported_steep() -> int:
    return steep(2)
    #      ^ d: tea/kettle.py:40


def count() -> int:
    global brews
    brews += 1
    return brews
    #      ^ d: tea/brewing.py:16


def counter():
    poured = 0

    def more() -> int:
        nonlocal poured
        poured += 1
        return poured
        #      ^ d: tea/brewing.py:39

    return more


def doubled(cups: list[int]) -> list[int]:
    return [cup * 2 for cup in cups]
    #       ^ d: tea/brewing.py:51


def first_cup() -> int:
    return cup
    #      ^ d: tea/brewing.py:17


def material(k: Kettle) -> str:
    return k.material + str(k.litres) + str(k.heat)
    #        ^ d: tea/kettle.py:21
    #                         ^ d: tea/kettle.py:25
    #                                         ^ d: tea/kettle.py:22


class TravelKettle(Kettle):
    def size(self) -> int:
        return self.litres + self.capacity
        #           ^ d: tea/kettle.py:25
        #                         ^ d: tea/kettle.py:14

    def pour(self) -> int:
        return super().pour() + Kettle.material.count("e")
        #              ^ d: tea/kettle.py:28
        #                              ^ d: tea/kettle.py:21


def timed(fn):
    @functools.wraps(fn)
    def inner(*args, **kwargs):
        return fn(*args, **kwargs)

    return inner


@timed
#^ d: tea/brewing.py:79
def brew(minutes: int) -> int:
    return chill(minutes)
    #      ^ d: picker tea/fast.py:1, tea/stale.py:6


def brew_twice() -> int:
    return brew(1) + brew(2)
    #      ^ d: tea/brewing.py:89


def compare(old: StaleKettle, new: Kettle) -> int:
    #            ^ d: tea/stale.py:1
    return old.pour() - new.pour()
    #          ^ d: tea/stale.py:2
    #                       ^ d: tea/kettle.py:28


def infuse() -> int:
    order = Infusion(leaves=3)
    #                ^ d: tea/kettle.py:46
    return order.leaves + len(order.cups)
    #            ^ d: tea/kettle.py:46
    #                               ^ d: tea/kettle.py:47


def serve(order: object) -> int:
    match order:
        case Infusion(leaves=leaves):
            return leaves
            #      ^ d: tea/brewing.py:116
        case [first, *rest]:
            return first + len(rest)
            #      ^ d: tea/brewing.py:119
            #                  ^ d: tea/brewing.py:119
        case {"heat": heat}:
            return heat
            #      ^ d: tea/brewing.py:123
        case steep:
            return steep
            #      ^ d: tea/brewing.py:126


def label(k: Kettle) -> str:
    return f"{k.material} kettle, {steep(1)} min"
    #           ^ d: tea/kettle.py:21
    #                              ^ d: tea/kettle.py:40


RECIPE = f"""
def chill(minutes): ...
class Infusion:
    leaves = {cup}
"""


def pot() -> int:
    teapot = Teapot()
    #        ^ d: tea/kettle.py:35
    return teapot.pour() + teapot.strength
    #             ^ d: tea/kettle.py:36
    #                             ^ d: none


def sweeten(cups: list[int]) -> list[int]:
    return list(map(lambda cup: cup + 1, cups))
    #                           ^ d: tea/brewing.py:153


STRENGTHS = [1, 2]


def strong() -> list[int]:
    return [s for s in STRENGTHS if s in STRENGTHS]
    #                  ^ d: tea/brewing.py:157
    #                                    ^ d: tea/brewing.py:157
    #                               ^ d: tea/brewing.py:161


def by_default() -> int:
    return (lambda n=brews: n)()
    #                ^ d: tea/brewing.py:16
