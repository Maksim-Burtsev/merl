class Tumbler:
    def pour(self) -> int:
        return 4


from cups import Tumbler as Glass

x = Tumbler()
x.pour()
# ^ d: cups_renamed.py:2
g = Glass()
g.pour()
# ^ d: cups.py:2
