class Tumbler:
    def pour(self) -> int:
        return 2


from cups import Tumbler

y = Tumbler()
#   ^ d: cups.py:1
y.pour()
# ^ d: cups.py:2
