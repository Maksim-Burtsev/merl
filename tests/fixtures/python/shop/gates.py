"""Rules of another file type or language stay out (review of #563)."""
from shop.dials import dial


def turn():
    # A stub's plain `def` among its overloads is no implementation (#338): the picker stays.
    return dial("k")
    #      ^ d: picker shop/dials.pyi:4, shop/dials.pyi:6, shop/dials.pyi:7


def launch():
    # An argument named like the function it is passed to is no Go parameter type (#536).
    Barge(Barge)
    #     ^ d: picker shop/hull.py:1, shop/tugs.py:1
