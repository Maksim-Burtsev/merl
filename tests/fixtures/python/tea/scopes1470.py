class Trivet:
    size = 1


def make_trivet():
    class Trivet:
        size = 2

    return Trivet()
    #      ^ d: tea/scopes1470.py:6


trivet = Trivet()
#        ^ d: tea/scopes1470.py:1


def garnish():
    return 1


def outer_garnish():
    def garnish():
        return 2

    return garnish()
    #      ^ d: tea/scopes1470.py:22


def plain_garnish():
    return garnish()
    #      ^ d: tea/scopes1470.py:17


def sip(cmd):
    sip_level = 5
    match cmd:
        case [sip_level]:
            return sip_level
            #      ^ d: tea/scopes1470.py:37
        case _:
            return sip_level
            #      ^ d: picker tea/scopes1470.py:35, tea/scopes1470.py:37
