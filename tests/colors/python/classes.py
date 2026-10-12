# Classes, methods, decorators, self and cls.
import dataclasses
import functools


@dataclasses.dataclass(frozen=True)
class Order(Base, metaclass=Meta):
    """An order of some items."""

    id: int
    items: list[str] = dataclasses.field(default_factory=list)
    _cache = {}

    def __init__(self, id: int, *args, **kwargs) -> None:
        super().__init__(*args, **kwargs)
        self.id = id
        self.total = 0.0
        self._seen: set[int] = set()

    def __repr__(self):
        return f"Order({self.id!r}, total={self.total:.2f})"

    @property
    def size(self) -> int:
        return len(self.items)

    @classmethod
    def empty(cls) -> "Order":
        return cls(0)

    @staticmethod
    def check(value: object) -> bool:
        return isinstance(value, Order) and value.id > 0

    @functools.cache
    async def fetch(self, client, *, retries: int = 3):
        async with client.session() as s:
            resp = await s.get(self.url, timeout=10)
        return resp.json()["items"]


class Empty:
    pass


order = Order(1)
order.items.append("x")
size = Order.empty().size
