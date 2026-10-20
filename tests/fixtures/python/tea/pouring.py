import logging
import os

import tea.brewer as kit
from tea.brewer import Brewer, Cup, Samovar, Strainer, Urn, make_brewer
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
    #         ^ d: tea/brewer.py:9; want none (#1151)


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
        #                 ^ d: picker shop/client.py:5, tea/brewer.py:18; want tea/brewer.py:18 (#1155)
    return total + cups.get("a").get("b")
    #                   ^ d: none
    #                            ^ d: picker shop/client.py:5, tea/brewer.py:18; want tea/brewer.py:18 (#1155)


def brew() -> int:
    cup = (
        Brewer()
        .leaves(3)
        #^ d: picker tea/brewer.py:49, tea/brewer.py:62, tea/kettle.py:46; want tea/brewer.py:49 (#1152)
        .temperature(90)
        #^ d: picker tea/brewer.py:53, tea/brewer.py:65; want tea/brewer.py:53 (#1152)
        .build()
        #^ d: picker tea/brewer.py:57, tea/brewer.py:68; want tea/brewer.py:57 (#1152)
    )
    return cup.sip() + cup.strength
    #          ^ d: picker tea/brewer.py:37, tea/brewer.py:71; want tea/brewer.py:37 (#1152)
    #                      ^ d: tea/brewer.py:34


def quick() -> int:
    return make_brewer().leaves(1).build().size + Brewer.classic().build().sip()
    #                    ^ d: tea/brewer.py:49
    #                              ^ d: picker tea/brewer.py:57, tea/brewer.py:68; want tea/brewer.py:57 (#1152)
    #                                      ^ d: picker tea/brewer.py:30, tea/brewing.py:68; want tea/brewer.py:30 (#1152)
    #                                                    ^ d: tea/brewer.py:46
    #                                                                      ^ d: picker tea/brewer.py:37, tea/brewer.py:71; want tea/brewer.py:37 (#1152)


def aliased(machine: Machine) -> int:
    return Maker().leaves(1).build().sip() + kit.make_brewer().build().sip() + machine.build().sip()
    #      ^ d: tea/brewer.py:41
    #              ^ d: tea/brewer.py:49
    #                                            ^ d: tea/brewer.py:75
    #                                                          ^ d: tea/brewer.py:57
    #                                                                                  ^ d: picker tea/brewer.py:57, tea/brewer.py:68; want tea/brewer.py:57 (#1153)


def again(brewer: Brewer) -> int:
    brewer = brewer.leaves(2)
    #        ^ d: tea/pouring.py:85
    #               ^ d: tea/brewer.py:49
    return brewer.build().sip()
    #      ^ d: picker tea/pouring.py:85, tea/pouring.py:86


def walk(cup: Cup) -> int:
    while cup.next:
        cup = cup.next
        #     ^ d: tea/pouring.py:93; want picker tea/pouring.py:93, tea/pouring.py:95 (#1154)
        #         ^ d: picker shop/warehouse.py:11, tea/brewer.py:31; want tea/brewer.py:31 (#1154)
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
    #      ^ d: tea/pouring.py:14; want tea/pouring.py:112 (#1156)

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
        #      ^ d: tea/pouring.py:132
        #                      ^ d: picker tea/stale.py:6, tea/kettle.py:40
    except ValueError as names:
        return len(str(names))
        #              ^ d: tea/pouring.py:139
