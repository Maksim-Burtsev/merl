from repos import UserRepository


def example_in_docstring(repo: UserRepository) -> None:
    """Forget a user.

    Example:
        from fakes import UserRepository
    """
    repo.delete_user(30)
