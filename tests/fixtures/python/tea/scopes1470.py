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


def render(kind):
    _ = str
    match kind:
        case "a":
            return _("A")
        case _:
            return _("other")
            #      ^ d: picker tea/scopes1470.py:46, tea/scopes1470.py:50


def cases(names):
    for name in names:
        case = dict(name,
            name)
        #   ^ d: picker tea/scopes1470.py:56, tea/scopes1470.py:57


def install():
    global relish

    def relish():
        return 3


def use_relish():
    return relish()
    #      ^ d: picker tea/scopes1470.py:65, tea/relish_other.py:1
