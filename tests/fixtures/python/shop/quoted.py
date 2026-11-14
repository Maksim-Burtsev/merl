class Stylus:
    def blot(self) -> str:
        return "stylus"


class Nib:
    def blot(self) -> str:
        return "nib"


def a(s: Stylus) -> str:
    return "s.blot()"
    #         ^ d: none


def b(s: Stylus) -> str:
    return f"s.blot() {s.blot()}"
    #          ^ d: none
    #                    ^ d: shop/quoted.py:2


def c(s: Stylus) -> str:
    return f"{{s.blot()}}" + "{s.blot()}"
    #            ^ d: none
    #                             ^ d: none


def d(s: Stylus) -> str:
    return f"{s.blot() + 'x'} s.blot()"
    #           ^ d: shop/quoted.py:2
    #                           ^ d: none


def e(s: Stylus) -> str:
    return """
    s.blot()
    # ^ d: none
    """


def f(s: Stylus) -> str:
    return "a\\" + s.blot() + 'it\'s' + s.blot()  # "
    #                ^ d: shop/quoted.py:2
    #                                     ^ d: shop/quoted.py:2


def g(s: Stylus) -> str:
    return rf"\{s.blot()}"
    #             ^ d: shop/quoted.py:2
