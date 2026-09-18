from collections.abc import Iterable

from repos import AuditLog, UserRepository


def load_repos() -> list[UserRepository]:
    return [UserRepository()]


def sweep(repos: list[UserRepository], logs: "tuple[AuditLog, ...]", user_id: int) -> None:
    for repo in repos:
        repo.delete_user(user_id)
    for log in logs:
        log.delete_user(user_id + 1)
    repos.delete_user(user_id + 2), "the list itself has no such member"


def sweep_loaded(user_id: int) -> None:
    loaded = load_repos()
    for repo in loaded:
        repo.delete_user(user_id + 3)


def sweep_keys(by_name: dict[str, UserRepository], user_id: int) -> None:
    for key in by_name:
        key.delete_user(user_id + 4), "a dict hands out its keys"


def sweep_pairs(repos: Iterable[UserRepository], user_id: int) -> None:
    for index, repo in enumerate(repos):
        repo.delete_user(user_id + 5)


def sweep_mixed(repos: list[UserRepository], flag: bool, user_id: int) -> None:
    if flag:
        repos = [AuditLog()]
    for repo in repos:
        repo.delete_user(user_id + 6)


def sweep_either(flag: bool, user_id: int) -> None:
    if flag:
        found: list[UserRepository] = []
    else:
        found: list[AuditLog] = []
    for repo in found:
        repo.delete_user(user_id + 7)
