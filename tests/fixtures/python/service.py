from repos import AuditLog, Notifier, UserRepository


class UserService:
    def __init__(self, repo: UserRepository, notifier: Notifier) -> None:
        self.repo = repo
        self.audit = AuditLog()
        self.notifier = notifier

    async def remove(self, user_id: int) -> None:
        user = self.repo.find_user(user_id)
        await self.repo.delete_user(user_id)
        self.audit.delete_user(user_id)
        self.notifier.send(f"removed {user['id']}")


class Services:
    def __init__(self, users: UserService) -> None:
        self.users = users


class App:
    def __init__(self, services: Services) -> None:
        self.services = services


async def handle_delete(app: App, user_id: int) -> None:
    await app.services.users.remove(user_id)


def cleanup(user_id: int) -> None:
    repo = AuditLog()

    async def purge() -> None:
        repo = UserRepository()
        await repo.delete_user(user_id)

    repo.delete_user(user_id)
