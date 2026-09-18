from functools import cached_property
from typing import Generic, TypeVar
from repos import AuditLog, UserRepository

T = TypeVar("T")


class UnitOfWork:
    def __init__(self) -> None:
        self.users = UserRepository()
        self.audit: AuditLog = AuditLog()


class Box(Generic[T]):
    def __init__(self, item: T) -> None:
        self.item = item


class Folder:
    def __init__(self, parent: "Folder") -> None:
        self.parent = parent

    def root(self) -> "Folder":
        return self


def make_uow() -> UnitOfWork:
    return UnitOfWork()


class Handler:
    def __init__(self, uow: UnitOfWork, box: Box[UserRepository]) -> None:
        self.uow = uow
        self.box = box

    async def delete_account(self, user_id: int, folder: Folder) -> None:
        await self.uow.users.delete_user(user_id)
        self.uow.audit.delete_user(user_id)
        await self.box.item.delete_user(user_id)
        users = AuditLog()
        await make_uow().users.delete_user(user_id)
        users.delete_user(user_id)
        folder.parent.parent.parent.parent.parent.root()
        folder.parent.parent.parent.parent.parent.parent.root()


class Registry:
    @cached_property
    def users(self) -> UserRepository:
        return UserRepository()

    @property
    def audit(self):
        return AuditLog()


class Admin:
    def __init__(self, registry: Registry) -> None:
        self.registry = registry

    def purge(self, user_id: int) -> None:
        self.registry.users.delete_user(user_id)
        self.registry.audit.delete_user(user_id)
