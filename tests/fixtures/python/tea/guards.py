import json
import sys

import textwrap


def shout(text: str) -> str:
    return text


json.shout = shout


class Lid:
    def lift(self) -> int:
        return 1


def open(path: str) -> Lid:
    return Lid()


class Teacosy:
    if sys.version_info >= (3, 8):

        def spill_over(self) -> int:
            return 1


class Ghost:
    def __getattr__(self, name: str) -> int:
        return 0


class Spirit:
    def float_up(self) -> int:
        return 1


def guarded(path: str) -> int:
    with open(path) as handle:
        handle.lift()
        #      ^ d: tea/guards.py:15
    cosy = Teacosy()
    cosy.spill_over()
    #    ^ d: tea/guards.py:26
    ghost = Ghost()
    ghost.float_up()
    #     ^ d: tea/guards.py:36
    json.shout("x")
    #    ^ d: tea/guards.py:7
    return len(textwrap.ribbon("x"))
    #                   ^ d: textwrap.py:1
