from repos import AuditLog, UserRepository


def cast(kind: type, row: object) -> AuditLog:
    return AuditLog()


def own(found: object, user_id: int) -> None:
    repo = cast(UserRepository, found)
    repo.delete_user(user_id)
