from cruets.cruet import Cruet
from cruets.phial import Phial
from cruets.vial import Flute, Vial


def pour(c: Cruet, p: Phial, o: object) -> None:
    if isinstance(c, Vial):
        c.uncork()
        # ^ d: cruets/vial.py:5
    if isinstance(c, Phial):
        c.stopper()
        # ^ d: cruets/phial.py:8
    elif isinstance(c, Vial):
        c.stopper()
        # ^ d: cruets/vial.py:8
    else:
        c.stopper()
        # ^ d: cruets/cruet.py:5
    if isinstance(o, Phial):
        o.uncork()
        # ^ d: cruets/phial.py:5
    c.uncork()
    # ^ d: cruets/cruet.py:2
    p.uncork()
    # ^ d: cruets/phial.py:5


def guard(c: Cruet) -> int:
    if not isinstance(c, Vial):
        return 0
    return c.stopper()
    #        ^ d: cruets/vial.py:8


def asserted(c: Cruet) -> int:
    assert isinstance(c, Phial)
    return c.stopper()
    #        ^ d: cruets/phial.py:8


def loose(c: Cruet, ok: bool) -> None:
    if not isinstance(c, Vial):
        print(ok)
        return
    c.uncork()
    # ^ d: cruets/cruet.py:2
    if isinstance(c, Phial) and ok:
        c.uncork()
        # ^ d: cruets/cruet.py:2
    v = c.stopper() if isinstance(c, Vial) else 0
    #     ^ d: cruets/cruet.py:5
    print(v)


def later(c: Cruet) -> None:
    if isinstance(c, Vial):

        def inner() -> None:
            c.uncork()
            # ^ d: cruets/cruet.py:2

        def other(c: Phial) -> None:
            c.uncork()
            # ^ d: cruets/phial.py:5

        inner()
        other(Phial())


def rebound(c: Cruet, p: Phial) -> None:
    if isinstance(c, Vial):
        c = p
        c.uncork()
        # ^ d: !jump


def own(f: Flute) -> None:
    if isinstance(f, Vial):
        f.uncork()
        # ^ d: cruets/vial.py:13


def wrapped(c: Cruet) -> None:
    if isinstance(
        c, Vial
    ):
        c.stopper()
        # ^ d: cruets/vial.py:8


def otherwise(c: Cruet) -> int:
    if isinstance(c, Vial):
        c.uncork()
    else:
        return 0
    return c.stopper()
    #        ^ d: cruets/vial.py:8


def noisy(c: Cruet) -> int:
    if isinstance(c, Phial):
        c.uncork()
    else:
        print(c)
        return 0
    return c.stopper()
    #        ^ d: cruets/cruet.py:5


class Rack:
    held: Cruet

    def __init__(self, held: Cruet) -> None:
        self.held = held

    def pour(self) -> None:
        if isinstance(self.held, Vial):
            self.held.stopper()
            #         ^ d: cruets/vial.py:8
        self.held.stopper()
        #         ^ d: !jump

    def swap(self, p: Phial) -> None:
        if isinstance(self.held, Vial):
            self.held = p
            self.held.uncork()
            #         ^ d: !jump


def reassigned(c: Cruet, xs: list[Cruet]) -> None:
    if isinstance(c, Vial):
        for c in xs:
            c.uncork()
            # ^ d: cruets/cruet.py:2


def outside(c: Cruet, ok: bool) -> None:
    if not isinstance(c, Vial):
        print(ok)
    c.uncork()
    # ^ d: cruets/cruet.py:2
    if isinstance(c, dict):
        c.uncork()
        # ^ d: cruets/cruet.py:2
    if isinstance(c, (Vial, Phial)):
        c.uncork()
        # ^ d: cruets/cruet.py:2
    if isinstance(c, Vial) or ok:
        c.uncork()
        # ^ d: cruets/cruet.py:2


def unrelated(p: Phial, both: Phial | Vial) -> None:
    if isinstance(p, Vial):
        p.uncork()
        # ^ d: cruets/phial.py:5
    if isinstance(both, Vial):
        both.uncork()
        #    ^ d: cruets/vial.py:5
