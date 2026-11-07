from dataclasses import dataclass

from cups import Tumbler, decant

early = Tumbler()
#       ^ d: cups.py:1


def later() -> object:
    return Tumbler()
    #      ^ d: cups_own.py:15


@dataclass
class Tumbler:
    size: int = 0


def decant() -> int:
    return 2


x = Tumbler()
#   ^ d: cups_own.py:15
decant()
# ^ d: cups_own.py:19
