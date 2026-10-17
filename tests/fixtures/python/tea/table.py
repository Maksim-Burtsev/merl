from tea.stale import *
from tea.kettle import *


def serve() -> int:
    return Kettle(1).pour() + steep(2) + Teapot().pour()
    #      ^ d: picker tea/kettle.py:20, tea/stale.py:1; want tea/kettle.py:20 (#980)
    #                ^ d: picker tea/brewing.py:73, tea/kettle.py:16, tea/kettle.py:28, tea/kettle.py:36, tea/stale.py:2; want tea/kettle.py:28 (#981)
    #                         ^ d: picker tea/kettle.py:40, tea/stale.py:6; want tea/kettle.py:40 (#980)
    #                                    ^ d: tea/kettle.py:35
    #                                             ^ d: picker tea/brewing.py:73, tea/kettle.py:16, tea/kettle.py:28, tea/kettle.py:36, tea/stale.py:2; want tea/kettle.py:36 (#981)
