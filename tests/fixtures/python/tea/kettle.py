"""A kettle and a teapot sharing `pour`.

A docstring that reads like code declares nothing:

def steep(minutes): ...
class Teapot:
    strength = 0
"""

from dataclasses import dataclass, field


class Vessel:
    capacity = 1

    def pour(self) -> int:
        return self.capacity


class Kettle(Vessel):
    material = "steel"
    heat = 0

    def __init__(self, litres: int) -> None:
        self.litres = litres
        self.heat = 90

    def pour(self) -> int:
        return self.litres

    def boil(self) -> int:
        return self.pour() + self.capacity + self.heat


class Teapot(Vessel):
    def pour(self) -> int:
        return 2


def steep(minutes: int) -> int:
    return minutes


@dataclass
class Infusion:
    leaves: int
    cups: list[int] = field(default_factory=list)
