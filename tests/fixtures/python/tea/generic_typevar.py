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


def wrapped[
    Leaf,
](item: Leaf) -> Leaf:
    #   ^ d: tea/generic_typevar.py:31
    print(Leaf)
    #     ^ d: tea/generic_typevar.py:31
    return item


class Wrapped[
    Leaf: int,
]:
    def put(self, item: Leaf) -> None:
        #               ^ d: tea/generic_typevar.py:40
        print(item)


type Pair[Leaf] = tuple[Leaf, Leaf]
#                       ^ d: tea/generic_typevar.py:47


type Twin[
    Leaf,
] = tuple[Leaf, Leaf]
#         ^ d: tea/generic_typevar.py:52


def after(item: Leaf) -> Leaf:
    #           ^ d: tea/generic_typevar.py:3
    return item


def wrapped_shadow[
    Leaf,
](item: Leaf) -> None:
    Leaf = len(str(item))
    print(Leaf)
    #     ^ d: tea/generic_typevar.py:65


def wrapped_param[
    Leaf,
](Leaf: int) -> None:
    print(Leaf)
    #     ^ d: tea/generic_typevar.py:72
