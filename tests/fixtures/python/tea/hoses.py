class Urn:
    def pour(self, cups):
        return cups

    def fill(self):
        return 0

    @property
    def level(self):
        return 0


class Hose(Urn):
    def __init__(self, fn):
        self.pour = fn
        self.level = 1

    def run(self):
        self.pour(1)
        #    ^ d: tea/hoses.py:15
        self.fill()
        #    ^ d: tea/hoses.py:5
        super().pour(1)
        #       ^ d: tea/hoses.py:2
        print(self.level)
        #          ^ d: tea/hoses.py:9


class Spigot(Urn):
    def run(self):
        self.pour(1)
        #    ^ d: tea/hoses.py:2


class Tap(Urn):
    def __init__(self, fn):
        if fn:
            self.pour = fn

    def run(self):
        self.pour(1)
        #    ^ d: tea/hoses.py:2


class Faucet(Urn):
    def swap(self, fn):
        self.pour = fn

    def run(self):
        self.pour(1)
        #    ^ d: tea/hoses.py:2


class Nozzle(Hose):
    def run(self):
        self.pour(2)
        #    ^ d: tea/hoses.py:15


def spray(fn):
    h = Hose(fn)
    h.pour(3)
    # ^ d: tea/hoses.py:15
    s = Spigot()
    s.pour(3)
    # ^ d: tea/hoses.py:2
