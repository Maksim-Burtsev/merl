from typing import cast

from cruets.cruet import Cruet
from cruets.phial import Phial
from cruets.vial import Vial


def matched(o: object) -> None:
    match o:
        case Vial() as c:
            c.uncork()
            # ^ d: cruets/vial.py:5
        case Phial() as c:
            c.stopper()
            # ^ d: cruets/phial.py:8
        case Vial() | Phial() as c:
            c.uncork()
            # ^ d: !jump
        case Cruet(lid=c):
            c.uncork()
            # ^ d: !jump
    c.uncork()
    # ^ d: !jump


def one_line(o: object) -> None:
    match o:
        case Vial() as c: c.uncork()
        #                   ^ d: cruets/vial.py:5
        case Phial() as c: c.stopper()
        #                    ^ d: cruets/phial.py:8


def recast(x: Cruet, m: Phial) -> None:
    y = cast(Vial, x)
    y.stopper()
    # ^ d: cruets/vial.py:8
    y = m
    y.stopper()
    # ^ d: cruets/phial.py:8


def branched(x: Cruet, m: Phial, ok: bool) -> None:
    y = cast(Vial, x)
    if ok:
        y = m
    y.stopper()
    # ^ d: !jump
