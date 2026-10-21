import logging
import os

import tea.brewer as kit
from tea.brewer import Brewer, Cup, Infuser, Samovar, Strainer, Urn, make_brewer
from tea.spout import Maker

if os.name == "nt":
    from tea.stale import steep
else:
    from tea.kettle import steep

log = logging.getLogger(__name__)
heat = 95
Machine = Brewer

# class Cup:
#     def sip(self) -> int: ...
RECIPE = "def make_brewer(): pass"


def names(raw: str, words: list[str]) -> list[str]:
    text = raw.strip()
    #          ^ d: none
    parts = text.split(",") + " ".join(words).split()
    #            ^ d: none
    #                                         ^ d: none
    log.info("names")
    #   ^ d: tea/brewer.py:15; want none (#1100)
    return [p.strip() for p in parts]
    #         ^ d: none


def opened(path: str) -> str:
    with open(path) as handle:
        return handle.read() + os.path.join("a", "b")
        #             ^ d: none
        #                              ^ d: none


def counts(cups: dict[str, Strainer]) -> int:
    total = 0
    for name, strainer in cups.items():
        total += strainer.get(name)
        #         ^ d: tea/pouring.py:43
        #                 ^ d: tea/brewer.py:18
    return total + cups.get("a").get("b")
    #                   ^ d: none
    #                            ^ d: tea/brewer.py:18


def brew() -> int:
    cup = (
        Brewer()
        .leaves(3)
        #^ d: tea/brewer.py:49
        .temperature(90)
        #^ d: tea/brewer.py:53
        .build()
        #^ d: tea/brewer.py:57
    )
    return cup.sip() + cup.strength
    #          ^ d: tea/brewer.py:37
    #                      ^ d: tea/brewer.py:34


def quick() -> int:
    return make_brewer().leaves(1).build().size + Brewer.classic().build().sip()
    #                    ^ d: tea/brewer.py:49
    #                              ^ d: tea/brewer.py:57
    #                                      ^ d: tea/brewer.py:30
    #                                                    ^ d: tea/brewer.py:46
    #                                                                      ^ d: tea/brewer.py:37


def aliased(machine: Machine) -> int:
    return Maker().leaves(1).build().sip() + kit.make_brewer().build().sip() + machine.build().sip()
    #      ^ d: tea/brewer.py:41
    #              ^ d: tea/brewer.py:49
    #                                            ^ d: tea/brewer.py:75
    #                                                          ^ d: tea/brewer.py:57
    #                                                                                  ^ d: tea/brewer.py:57


def again(brewer: Brewer) -> int:
    brewer = brewer.leaves(2)
    #        ^ d: tea/pouring.py:85
    #               ^ d: tea/brewer.py:49
    return brewer.build().sip()
    #      ^ d: picker tea/pouring.py:85, tea/pouring.py:86


def walk(cup: Cup) -> int:
    while cup.next:
        cup = cup.next
        #     ^ d: picker tea/pouring.py:93, tea/pouring.py:95
        #         ^ d: tea/brewer.py:31
    return cup.size


def each(brewers: list[Brewer]) -> int:
    for brewer in brewers:
        brewer.leaves(1)
        #      ^ d: tea/brewer.py:49
    if (cup := brewer.build()) is not None:
        return cup.sip()
        #      ^ d: tea/pouring.py:105
    return 0


class Pot:
    heat = 80
    hot = [heat for _ in range(2)]
    #      ^ d: tea/pouring.py:14
    warm = heat + 1
    #      ^ d: tea/pouring.py:112
    cold = [
        heat
        #^ d: tea/pouring.py:14
        for _ in range(2)
    ]

    def boil(self) -> int:
        return heat
        #      ^ d: tea/pouring.py:14


class Kiosk(Samovar):
    def serve(this) -> int:
        return this.warm() + this.degrees() + Urn().warm()
        #           ^ d: tea/brewer.py:80
        #                         ^ d: tea/brewer.py:88
        #                                           ^ d: tea/brewer.py:96


def outer() -> int:
    def make_brewer() -> int:
        return 3

    try:
        return make_brewer() + steep(1)
        #      ^ d: tea/pouring.py:137
        #                      ^ d: picker tea/stale.py:6, tea/kettle.py:40
    except ValueError as names:
        return len(str(names))
        #              ^ d: tea/pouring.py:144


def mixed(brewers: list[Brewer], raw: str) -> int:
    more = brewers + brewers
    both = raw.split() + brewers
    pieces = raw.split() + " a b".split()
    for piece in pieces:
        piece.strip()
        #     ^ d: none
    return len([b.leaves(1) for b in more]) + len([s.leaves(1) for s in both])
    #                  ^ d: picker tea/brewer.py:49, tea/brewer.py:62, tea/kettle.py:46
    #                                                 ^ d: picker tea/brewer.py:49, tea/brewer.py:62, tea/kettle.py:46


class Lab:
    @classmethod
    def setup(cls) -> None:
        cls.kit = Brewer()

    def run(self) -> int:
        return self.kit.leaves(1).build().sip()
        #           ^ d: tea/pouring.py:164
        #               ^ d: tea/brewer.py:49


def clean(lines: list[str], items: list[str]) -> int:
    for line in lines:
        line = line.strip()
        #      ^ d: tea/pouring.py:173
    for n, item in enumerate(items):
        if item:
            item = item.lower()
            #      ^ d: tea/pouring.py:176
    for cup in lines:
        while cup:
            cup = cup[1:]
            #     ^ d: picker tea/pouring.py:180, tea/pouring.py:182
    return len(lines) + n


def twice(raw: str) -> int:
    once = raw.strip()
    again = once.strip()
    return len(again.split())
    #                ^ d: none


class Rig:
    kit_class = Brewer

    @classmethod
    def setup(cls) -> None:
        cls.rig = cls.kit_class()


class InfuserRig(Rig):
    kit_class = Infuser


class LeftRig(Rig):
    pass


class Trial(InfuserRig):
    def run(self) -> int:
        return self.rig.leaves(1)
        #               ^ d: tea/brewer.py:62


class PlainTrial(Rig):
    def run(self) -> int:
        return self.rig.leaves(1).build().sip()
        #               ^ d: tea/brewer.py:49


class Both(LeftRig, InfuserRig):
    def run(self) -> int:
        return self.rig.leaves(1)
        #               ^ d: tea/brewer.py:62


class MakerRig:
    maker = make_brewer

    @classmethod
    def setup(cls) -> None:
        cls.made = cls.maker()

    def run(self) -> int:
        return self.made.leaves(1)
        #                ^ d: picker tea/brewer.py:49, tea/brewer.py:62, tea/kettle.py:46
