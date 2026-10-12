from abc import ABC

import six
from rpc import Remote


class Dial:
    def twiddle(self) -> None:
        pass


class Abstract(ABC):
    pass


class Metaclassed(six.with_metaclass(type, object), Remote):
    pass


class Loose(Unbound, Remote):
    pass


class Wired(Remote):
    pass


def turn() -> None:
    a = Abstract()
    a.twiddle()
    # ^ d: ancestry.py:8
    m = Metaclassed()
    m.twiddle()
    # ^ d: ancestry.py:8
    lo = Loose()
    lo.twiddle()
    #  ^ d: ancestry.py:8
    w = Wired()
    w.twiddle()
    # ^ d: none
