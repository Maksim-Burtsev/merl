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


def documented(user_id: int) -> None:
    """Deletes through the module's ledger.

    Example:

        ledger = UserRepository()
    """
    ledger.delete_user(user_id + 5)


def one_line(fresh: bool) -> None:
    if fresh: ledger = UserRepository()
    ledger.delete_user(10)


def one_line_unread(fresh: bool) -> None:
    if fresh: ledger = UserRepository()
    else: ledger = open("ledger")
    ledger.delete_user(11)


def after_a_semicolon() -> None:
    count = 1; ledger = UserRepository()
    ledger.delete_user(12 + count)


def behind_a_loop(names: dict[str, int]) -> None:
    for name in names["a:b"]: ledger = UserRepository()
    ledger.delete_user(13)


def annotated_behind_a_with() -> None:
    with open("ledger") as source: ledger: UserRepository = source
    ledger.delete_user(14)


def chained() -> None:
    first = ledger = UserRepository()
    ledger.delete_user(15 + len(first.users))


def imported_in_a_try() -> None:
    try: from fakes import ledger
    except ImportError: pass
    ledger.delete_user(16)


def only_compared(flag: bool) -> None:
    if ledger == flag: print(ledger)
    ledger.delete_user(17)


class Vault:
    def __init__(self, cold: bool) -> None:
        self.ledger = AuditLog()
        if cold: self.ledger = UserRepository()

    def purge(self) -> None:
        self.ledger.delete_user(18)


def behind_a_wrapped_header(fresh: bool, cold: bool) -> None:
    if (fresh and
            cold): ledger = UserRepository()
    ledger.delete_user(19)


def behind_a_wrapped_call(fresh: bool) -> None:
    if bool(
            fresh): ledger = UserRepository()
    ledger.delete_user(20)


def a_keyword_on_its_own_line() -> None:
    print("purge",
          end = "\n")
    dict(users = 1,
         ledger = UserRepository())
    ledger.delete_user(21)
