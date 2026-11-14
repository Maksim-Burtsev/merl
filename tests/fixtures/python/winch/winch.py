from winch.pawl import Pawl


class Winch:
    def __init__(self):
        self._slack = Pawl()

    @property
    def slack(self) -> Pawl:
        return self._slack

    @slack.setter
    def slack(self, v):
        self._slack = v

    @slack.deleter
    def slack(self):
        del self._slack

    @property
    def width(self) -> int:
        return 1

    def grow(self):
        self.slack.click()
        #    ^ d: winch/winch.py:9
        self.slack = Pawl()
        #    ^ d: winch/winch.py:13
