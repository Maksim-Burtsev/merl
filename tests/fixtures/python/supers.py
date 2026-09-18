import json
from typing import Generic, TypeVar

from repos import AuditLog

T = TypeVar("T")


class Archive:
    label: str = "archive"

    def __init__(self, audit: AuditLog) -> None:
        self.audit = audit

    def store(self, item: int) -> None:
        pass

    def flush(self) -> None:
        pass


class ColdArchive(Archive):
    def __init__(self, audit: AuditLog) -> None:
        super().__init__(audit)

    def store(self, item: int) -> None:
        super().store(item)


class GlacierArchive(ColdArchive, Generic[T]):
    def store(self, item: int) -> None:
        super().store(item + 1)
        super().flush()
        print(super().label)

    def flush(self) -> None:
        def later() -> None:
            super().flush(), "no arguments to find in here"

        later()


class Stamped:
    def store(self, item: int) -> None:
        pass

    def stamp(self) -> None:
        pass


class Mixed(Stamped, ColdArchive):
    def store(self, item: int) -> None:
        super().store(item + 2)
        super().stamp()
        super().flush(), "one base leads to it"


class Left(Archive):
    pass


class Right(Archive):
    def flush(self) -> None:
        pass


class Diamond(Left, Right):
    def flush(self) -> None:
        super().flush(), "Right.flush, which a walk by depth passes"


class Wire(json.JSONEncoder, Archive):
    def store(self, item: int) -> None:
        super().store(item + 3)

    def default(self, o: object) -> object:
        return super().default(o)


class Ledger(Archive):
    def close(self) -> None:
        super().audit.store(0)
