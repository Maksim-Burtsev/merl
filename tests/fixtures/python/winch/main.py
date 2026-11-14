from winch.pawl import Pawl
from winch.winch import Winch


def run(w: Winch):
    print(w.slack)
    #       ^ d: winch/winch.py:9
    w.slack = Pawl()
    # ^ d: winch/winch.py:13
    w.slack.click()
    # ^ d: winch/winch.py:9
    #       ^ d: winch/pawl.py:2
    w.slack += 1
    # ^ d: picker winch/winch.py:9, winch/winch.py:13, winch/winch.py:17
    a, w.slack = 1, Pawl()
    #    ^ d: picker winch/winch.py:9, winch/winch.py:13, winch/winch.py:17
    del w.slack
    #     ^ d: picker winch/winch.py:9, winch/winch.py:13, winch/winch.py:17
    print(w.width)
    #       ^ d: winch/winch.py:21


x = Winch()
x.slack.click()
# ^ d: winch/winch.py:9
x.slack = Pawl()
# ^ d: winch/winch.py:13


def check(w: Winch):
    if w.slack:
        # ^ d: winch/winch.py:9
        return {"slack": w.slack}
        #                  ^ d: picker winch/winch.py:9, winch/winch.py:13, winch/winch.py:17
    return None
