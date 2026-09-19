from dataclasses import dataclass
from enum import Enum

from repos import AuditLog
from store import open_session

conn = AuditLog()
wrapped = AuditLog()


class Limits:
    MAX_USERS = 10
    timeout: int = 30

    def check(self) -> None:
        pass


class Tight(Limits):
    timeout = 5

    def __init__(self) -> None:
        self.count = 0


class Color(Enum):
    RED = 1
    GREEN = 2


class Shade(Enum):
    RED = "dark"


@dataclass
class Point:
    x: int
    label: str = ""


def paint(p: Point) -> None:
    Limits.MAX_USERS
    Limits.timeout
    Color.RED
    Shade.RED
    Point.label
    p.label
    Limits.missing
    Tight.MAX_USERS
    Tight.timeout
    Tight.count
    Tight.check
    Color.RED.value
    scan.cache


def inner() -> None:
    class Color:
        RED = 3

    Color.RED  # the class of this function


def hidden(Color: Shade, Limits) -> None:
    Color.RED  # a parameter
    Limits.MAX_USERS  # a parameter


def scan() -> None:
    with open_session() as conn, open("x") as handle:
        conn.delete_user(1)
        handle.read()
    with (
        open_session() as wrapped,
    ):
        wrapped.delete_user(2)


class Root:
    LEVEL = 0


class Left(Root):
    pass


class Right(Root):
    LEVEL = 2


class Diamond(Left, Right):
    pass


def diamond() -> int:
    # Python reads `Right.LEVEL`; a walk by depth would pass it.
    return Diamond.LEVEL


class Plain:
    RANK = 1
    Meta = object
    CODE = 0

    def check(self) -> bool:
        return True


class Unread(Plain):
    RANK, OTHER = 2, 3
    if RANK:
        def check(self) -> bool:
            return False

    class Meta:
        pass

    for CODE in (1, 2):
        pass


def unread() -> None:
    # Every one is `Unread`'s own, in a shape the rules do not read.
    Unread.RANK
    Unread.check
    Unread.Meta
    Unread.CODE


class Noted(Plain):
    x = 1
# a comment at column 0 ends no class body
    class Meta:
        pass


class Queried(Plain):
    QUERY = """
select 1
"""
    RANK, y = 4, 5


class Short(Plain): CODE, z = 6, 7


def cut_short() -> None:
    Noted.Meta
    Queried.RANK
    Short.CODE


class Flush(
Plain,
):
    CODE, w = 8, 9


def flush() -> None:
    Flush.CODE
