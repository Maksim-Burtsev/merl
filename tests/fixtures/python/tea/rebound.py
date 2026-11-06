from tea.blends import infuse

LEAVES = 1


def own(infuse): return infuse
#                       ^ d: tea/rebound.py:6


def preset(LEAVES = 2): return LEAVES
#                              ^ d: tea/rebound.py:10


def after(): infuse = 1; return infuse
#                               ^ d: tea/rebound.py:14


def unused(): f = lambda infuse: 0; return infuse
#                                          ^ d: tea/blends.py:1


def default(a = infuse): return a
#               ^ d: tea/blends.py:1


def called(xs):
    return infuse(1) + (lambda infuse: infuse)(2)
#          ^ d: tea/blends.py:1
#                                      ^ d: tea/rebound.py:27


def listed(xs):
    return [infuse for infuse in xs] + [infuse(1)]
#           ^ d: tea/rebound.py:33
#                                       ^ d: tea/blends.py:1


def keyed(xs):
    return {LEAVES: 1 for LEAVES in xs} | {LEAVES: 2}
#           ^ d: tea/rebound.py:39
#                                          ^ d: tea/rebound.py:3


def iterated(xs):
    return [infuse for infuse in infuse(xs)]
#                                ^ d: tea/blends.py:1


def passed(xs):
    return f(lambda infuse: 1, infuse)
#                              ^ d: tea/blends.py:1


[print(cup) for cup in [0]]
#      ^ d: tea/rebound.py:54
cup = 2
