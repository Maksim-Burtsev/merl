from functools import cache

from repos import AuditLog, UserRepository


class Depot:
    def __init__(self) -> None:
        self.people = UserRepository()

    def people_repo(self) -> UserRepository:
        return self.people

    def trail(self):
        if self.people:
            return AuditLog()
        return AuditLog()

    def either_one(self):
        if self.people:
            return AuditLog()
        return UserRepository()

    def maybe_trail(self):
        if self.people:
            return AuditLog()
        return None


def open_depot() -> Depot:
    return Depot()


@cache
def shared_trail():
    return AuditLog()


def first(depot: Depot, user_id: int) -> None:
    repo = depot.people_repo()
    repo.delete_user(user_id)
    trail = depot.trail()
    trail.delete_user(user_id + 1)
    open_depot().people.delete_user(user_id + 2)
    Depot().people.delete_user(user_id + 3)
    either = depot.either_one()
    either.delete_user(user_id + 4)
    maybe = depot.maybe_trail()
    maybe.delete_user(user_id + 5)
    shared = shared_trail()
    shared.delete_user(user_id + 6)
    open_depot().people_repo().delete_user(user_id + 7)


def second(depot, user_id: int) -> None:
    repo = depot.people_repo()
    repo.delete_user(user_id + 8)


def retry(times: int, wrap: type):
    return lambda made: made


@retry(
    times=3,
    wrap=AuditLog,
)
def make_wrapped():
    return UserRepository()


def third(user_id: int) -> None:
    wrapped = make_wrapped()
    wrapped.delete_user(user_id + 9)


def fourth(open_depot, user_id: int) -> None:
    open_depot().people.delete_user(user_id + 10)
    made = open_depot()
    made.people.delete_user(user_id + 11)


class SubDepot(Depot):
    pass


class Yard:
    def __init__(self, depot: Depot) -> None:
        self.depot = depot

    def sweep(self, sub: SubDepot, user_id: int) -> None:
        inherited = sub.people_repo()
        inherited.delete_user(user_id + 12)
        through = self.depot.people_repo()
        through.delete_user(user_id + 13)
        self.depot.people_repo().delete_user(user_id + 14)
        ahead = behind.people_repo()
        behind = ahead.people_repo()
        ahead.delete_user(user_id + 15)


def pick_inline(flag: bool):
    if flag: return AuditLog()
    return UserRepository()


def fifth(user_id: int) -> None:
    picked = pick_inline(True)
    picked.delete_user(user_id + 16)
