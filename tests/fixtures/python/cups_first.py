class Tumbler:
    def pour(self) -> int:
        return 2


from cups import Tumbler

y = Tumbler()
#   ^ d: cups.py:1
y.pour()
# ^ d: cups_first.py:2; want cups.py:2 (#1407)
