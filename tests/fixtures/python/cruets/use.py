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
    # ^ d: !jump
    if isinstance(c, Phial) and ok:
        c.uncork()
        # ^ d: !jump
    v = c.stopper() if isinstance(c, Vial) else 0
    #     ^ d: !jump
    print(v)


def later(c: Cruet) -> None:
    if isinstance(c, Vial):

        def inner() -> None:
            c.uncork()
            # ^ d: !jump

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
    #        ^ d: !jump


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
