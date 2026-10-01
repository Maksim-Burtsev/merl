"""A function's own `def`, a parameter of a wrapped signature, a `def` nested in a function
beside a method of its name, and an `@overload` set (#338)."""
from shop.gauges import gauge
from shop.meters import reading


def wrap_gift():
    def fold(x):
        return x

    return fold(1)
    #      ^ d: shop/nesting.py:8
    #      status: fold → wrap_gift.fold (local)


def unwrap_gift():
    def fold(y):
        return y

    return fold(2)


def label_box(
    lid: int,
    produce_archive: bool,
    filename: str,
) -> None:
    label_box(lid, produce_archive=produce_archive, filename=filename)
    #                              ^ d: shop/nesting.py:25


def plan_route():
    def enqueue():
        return 1

    return enqueue


class Van:
    def enqueue(self):
        return 2


def start_route(van):
    van.enqueue()
    #   ^ d: shop/nesting.py:40
    #   status: enqueue → Van.enqueue (by name, 1 match)


LIMIT = gauge("LIMIT", 80)
#       ^ d: shop/gauges.py:12
#       status: gauge: via import shop/gauges.py
LEVEL = reading("LEVEL")
#       ^ d: picker shop/meters.pyi:4, shop/meters.pyi:6
