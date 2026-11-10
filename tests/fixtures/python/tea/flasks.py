# A chain of declared returns through `-> Self` (#1469): the link keeps the receiver's own type.
from typing import Self


class Flask:
    def tip(self) -> int:
        return 1

    def sized(self, n: int) -> Self:
        return self

    def named(self, s: str) -> "Flask":
        return self


class TallFlask(Flask):
    def tip(self) -> int:
        return 3


class Gourd:
    def tip(self) -> int:
        return 2

    def sized(self, n: int) -> "Gourd":
        return self


def pour(tall: TallFlask, by_name: dict[str, Flask]) -> int:
    a = Flask().sized(2).named("a").tip()
    #                               ^ d: tea/flasks.py:6
    b = tall.sized(2).sized(3).tip()
    #                          ^ d: tea/flasks.py:17
    c = Gourd().sized(2).sized(3).tip()
    #                             ^ d: tea/flasks.py:22
    d = by_name["a"].tip()
    #                ^ d: picker tea/flasks.py:6, tea/flasks.py:17, tea/flasks.py:22; want tea/flasks.py:6 (#1469)
    return a + b + c + d
