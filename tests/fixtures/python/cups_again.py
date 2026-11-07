from cups import Tumbler

before = Tumbler()
before.pour()
#      ^ d: cups.py:2


class Tumbler:
    def pour(self) -> int:
        return 3


def later(t: Tumbler) -> int:
    return t.pour()
    #        ^ d: cups_again.py:9


after = Tumbler()
after.pour()
#     ^ d: cups_again.py:9
