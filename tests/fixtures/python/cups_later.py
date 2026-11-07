class Tumbler:
    def pour(self) -> int:
        return 2


early = Tumbler()
early.pour()
#     ^ d: cups_later.py:2
early_typed: Tumbler = early
early_typed.pour()
#           ^ d: cups_later.py:2

from cups import Tumbler


def later() -> int:
    z = Tumbler()
    return z.pour()
    #        ^ d: cups.py:2


def typed(t: Tumbler) -> int:
    return t.pour()
    #        ^ d: cups.py:2


w: Tumbler = Tumbler()
w.pour()
# ^ d: cups.py:2
Tumbler.pour(w)
#       ^ d: cups.py:2
