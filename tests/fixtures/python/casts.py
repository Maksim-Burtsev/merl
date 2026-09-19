import typing
from typing import cast

from repos import AuditLog, UserRepository


def first(found: object, user_id: int) -> None:
    repo = cast(UserRepository, found)
    repo.delete_user(user_id)
    audit = typing.cast("AuditLog", found)
    audit.delete_user(user_id + 1)
    cast(UserRepository, found).delete_user(user_id + 2)
    missing = cast("Missing", found)
    missing.delete_user(user_id + 3)
    loose = cast(typing.Any, found)
    loose.delete_user(user_id + 4)
