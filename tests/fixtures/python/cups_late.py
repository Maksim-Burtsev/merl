from cups import Goblet


class Goblet(Goblet):
#            ^ d: cups.py:18
    pass


x = Goblet()
#   ^ d: cups_late.py:4


def rinse() -> int:
    return 0


try:
    from cups import rinse
except ImportError:
    pass

rinse()
# ^ d: cups.py:10


def swirl() -> int:
    return 0


def brew() -> int:
    from cups import swirl

    return swirl()
    #      ^ d: cups.py:14


def serve() -> object:
    from cups import Chalice

    return Chalice()
    #      ^ d: cups.py:22


class Chalice:
    pass


def cups() -> int:
    return 0


import cups

cups.decant()
# ^ d: cups.py:1
