from typing import TypeVar

Leaf = TypeVar("Leaf")


def own[Leaf](item: Leaf) -> Leaf:
    #               ^ d: tea/generic_typevar.py:6
    print(Leaf)
    #     ^ d: tea/generic_typevar.py:6
    return item


def shadow[Leaf](item: Leaf) -> None:
    Leaf = len(str(item))
    print(Leaf)
    #     ^ d: tea/generic_typevar.py:14


def plain(item: Leaf) -> Leaf:
    #           ^ d: tea/generic_typevar.py:3
    return item


class Basket[Leaf]:
    def put(self, item: Leaf) -> None:
        #               ^ d: tea/generic_typevar.py:24
        print(item)
