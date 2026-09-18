from repos import AuditLog, UserRepository

ledger = AuditLog()


def save() -> None:
    pass


def handler(save: UserRepository) -> None:
    save.find_user(1)


def rotate(user_id: int) -> None:
    ledger = UserRepository()
    ledger.find_user(user_id)

    def later() -> None:
        ledger.find_user(user_id + 1)


def audit_only(user_id: int) -> None:
    ledger.delete_user(user_id + 2)


def twice(flag: bool) -> None:
    if flag:
        ledger = UserRepository()
    else:
        ledger = AuditLog()
    ledger.delete_user(3)


def hidden(ledger, user_id: int) -> None:
    ledger.delete_user(user_id + 4)
