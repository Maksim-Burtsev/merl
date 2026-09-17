from repos import AuditLog, UserRepository
from store import connect


def make_repo() -> UserRepository:
    return UserRepository()


def make_audit():
    return AuditLog()


def rotate(user_id: int) -> None:
    repo = make_repo()
    repo.delete_user(user_id)
    audit = make_audit()
    audit.delete_user(user_id)
    session = connect()
    session.close()


def forget(repo, user_id: int):
    repo.find_user(user_id)
    return repo.delete_user(user_id)
