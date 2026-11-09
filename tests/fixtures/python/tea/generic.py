from tea.brewer import Cup


def serve[T: Cup](item: T, cup: Cup, width: int) -> str:
    item
    # ^ d: tea/generic.py:4
    cup.sip()
    #   ^ d: tea/brewer.py:37
    return f"{cup!r} {width}"
    #         ^ d: tea/generic.py:4
    #                 ^ d: tea/generic.py:4


def plain(item: int, cup: Cup, width: int) -> str:
    item
    # ^ d: tea/generic.py:14
    cup.sip()
    #   ^ d: tea/brewer.py:37
    return f"{cup!r} {width}"
    #         ^ d: tea/generic.py:14
    #                 ^ d: tea/generic.py:14


def pick[T: (str, bytes)](first: T, cup: Cup) -> T:
    cup.sip()
    #   ^ d: tea/brewer.py:37
    return first
    #      ^ d: tea/generic.py:24


async def nested[K, V: dict[str, list[Cup]]](key: K, cup: Cup) -> None:
    cup.sip()
    #   ^ d: tea/brewer.py:37
    print(key)
    #     ^ d: tea/generic.py:31


class Shelf[T]:
    def put[U](self, cup: Cup, extra: U) -> T:
        cup.sip()
        #   ^ d: tea/brewer.py:37
        return extra
        #      ^ d: tea/generic.py:39


def bound[T: Cup](item: T) -> T:
    #                   ^ d: tea/generic.py:46
    return item
    #      ^ d: tea/generic.py:46


class Rack[T: Cup]:
    def take(self, held: T) -> T:
        #                ^ d: tea/generic.py:52
        return held


type Pair[T] = tuple[T, T]
#                    ^ d: tea/generic.py:58


def shadowed[T](item: T) -> None:
    T = len(str(item))
    print(T)
    #     ^ d: tea/generic.py:63


def unlisted(item: int) -> None:
    print(T)
    #     ^ d: picker chains.py:5, supers.py:6


def named[Cup](c: Cup) -> None:
    c.sip()
    # ^ d: picker tea/brewer.py:37, tea/brewer.py:71


class Holder[Cup]:
    def put(self, c: list[Cup]) -> None:
        for one in c:
            one.sip()
            #   ^ d: picker tea/brewer.py:37, tea/brewer.py:71


async def gather[*Ts, **P](*parts: *Ts, call: Callable[P, int]) -> None:
    #                               ^ d: tea/generic.py:85
    #                                                  ^ d: tea/generic.py:85
    return None


class Beaker[T](Cup):
    def fill(self, held: T) -> T:
        #                ^ d: tea/generic.py:91
        return held


def one[T: (str, bytes)](first: T) -> T: return first
#                                               ^ d: tea/generic.py:97


def make[T]() -> Cup:
    return Cup(1)


made = make()
made.sip()
#    ^ d: tea/brewer.py:37


class Mug:
    pass


class Box[T: (Cup, Mug)]:
    pass


Box().sip()
#     ^ d: none


pick(first="a", cup=Cup(1))
#    ^ d: tea/generic.py:24


def back[Cup](c: Cup) -> Cup:
    return c


back(3).sip()
#       ^ d: picker tea/brewer.py:37, tea/brewer.py:71


def wrapped[T](
    T,
):
    return T
    #      ^ d: tea/generic.py:135


class Jar[T: (str, bytes)](
    Cup,
):
    lid = Cup(1)


Jar().lid.sip()
#         ^ d: tea/brewer.py:37
