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
