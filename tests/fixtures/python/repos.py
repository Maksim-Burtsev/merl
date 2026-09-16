from typing import Protocol


class UserRepository:
    def find_user(self, user_id: int) -> dict:
        return {"id": user_id}

    async def delete_user(self, user_id: int) -> None:
        pass


class AuditLog:
    def delete_user(self, user_id: int) -> None:
        pass


class Notifier(Protocol):
    def send(self, text: str) -> None: ...


class EmailNotifier(Notifier):
    def send(self, text: str) -> None:
        pass


class SmsNotifier(Notifier):
    def send(self, text: str) -> None:
        pass
