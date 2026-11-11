from typing import Tuple


class Platter:
    def dollop(self): ...


class Saucepan:
    def dollop(self): ...


class Cauldron:
    def dollop(self): ...


def serve_two() -> tuple[Platter, Saucepan]:
    return Platter(), Saucepan()


def serve_old() -> Tuple[Saucepan, Platter]:
    return Saucepan(), Platter()


def serve_many() -> tuple[Platter, ...]:
    return (Platter(),)


def serve_bare():
    return Platter(), Saucepan()


def unpack():
    first, second = serve_two()
    first.dollop()
    #     ^ d: shop/servings.py:5
    second.dollop()
    #      ^ d: shop/servings.py:9
    (left, right) = serve_old()
    right.dollop()
    #     ^ d: shop/servings.py:5
    near, far = Platter(), Saucepan()
    far.dollop()
    #   ^ d: shop/servings.py:9


def unsure(other: Cauldron):
    one, two = serve_bare()
    two.dollop()
    #   ^ d: picker shop/servings.py:5, shop/servings.py:9, shop/servings.py:13
    head, *rest = serve_two()
    head.dollop()
    #    ^ d: !jump
    only, extra = serve_many()
    extra.dollop()
    #     ^ d: !jump
    spare, other = serve_two()
    other.dollop()
    #     ^ d: !jump
