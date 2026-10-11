# Type hints, generics and protocols.
from typing import Callable, Generic, Optional, Protocol


class Store(Generic[T], Protocol):
    def get(self, key: str, default: Optional[T] = None) -> T | None: ...

    def items(self) -> Callable[[int, str], list[dict[str, bytes]]]: ...


type Pair[K, V] = tuple[K, V]


def first[T](xs: list[T]) -> T:
    return xs[0]


def walk(node: "Node", depth: int = 0, *rest: tuple, **opts: bool) -> None:
    x: float = 1.0
    y: frozenset[bytearray] = frozenset()
    print(type(node), id(node), hash(x), len(y), sum([1, 2]), max(1, 2), min(3, 4))
    print(Exception, ValueError, StopIteration, KeyboardInterrupt)
