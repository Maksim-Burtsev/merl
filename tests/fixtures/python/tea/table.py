from tea.stale import *
from tea.kettle import *


def serve() -> int:
    return Kettle(1).pour() + steep(2) + Teapot().pour()
    #      ^ d: tea/kettle.py:20
    #                ^ d: tea/kettle.py:28
    #                         ^ d: tea/kettle.py:40
    #                                    ^ d: tea/kettle.py:35
    #                                             ^ d: tea/kettle.py:36


def poured() -> int:
    teapot = Teapot()
    return teapot.pour()
    #             ^ d: tea/kettle.py:36
