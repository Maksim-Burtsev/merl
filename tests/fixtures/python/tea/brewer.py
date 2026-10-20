"""The third corner of #1126: a builder, and namesakes of the standard library's members."""

from __future__ import annotations

import abc


class Strainer:
    def strip(self) -> int:
        return 1

    def split(self, n: int) -> list[int]:
        return [n]

    def info(self, msg: str) -> None:
        print(msg)

    def get(self, key: str) -> int:
        return len(key)

    def read(self) -> str:
        return ""

    def join(self, *parts: str) -> str:
        return "".join(parts)


class Cup:
    def __init__(self, size: int) -> None:
        self.size = size
        self.next: Cup | None = None

    @property
    def strength(self) -> int:
        return self.size * 2

    def sip(self) -> int:
        return self.size


class Brewer:
    def __init__(self) -> None:
        self.cups = 0

    @classmethod
    def classic(cls) -> Brewer:
        return cls().leaves(2)

    def leaves(self, n: int) -> Brewer:
        self.cups += n
        return self

    def temperature(self, degrees: int) -> Brewer:
        self.cups += degrees // 100
        return self

    def build(self) -> Cup:
        return Cup(self.cups)


class Infuser:
    def leaves(self, n: int) -> int:
        return n

    def temperature(self, degrees: int) -> int:
        return degrees

    def build(self) -> int:
        return 0

    def sip(self) -> int:
        return 0


def make_brewer() -> Brewer:
    return Brewer()


class Warming(abc.ABC):
    def warm(self) -> int:
        return self.degrees() + 1

    @abc.abstractmethod
    def degrees(self) -> int: ...


class Samovar(Warming):
    def degrees(self) -> int:
        return 90


class Urn(Warming):
    def degrees(self) -> int:
        return 70

    def warm(self) -> int:
        return super().warm() * 2
