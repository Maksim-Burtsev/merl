class Lamp:
    glow = 1

    def glow(self):
        return 2


def light(lamp: Lamp):
    return lamp.glow()
    #           ^ d: tea/lamp.py:4
