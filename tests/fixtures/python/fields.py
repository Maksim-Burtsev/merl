from repos import AuditLog, UserRepository


class Base:
    def __init__(self) -> None:
        self.audit = AuditLog()


class Issue(Base):
    title: str
    poster_id: int = 0

    def __init__(self, repo: UserRepository) -> None:
        super().__init__()
        self.repo = repo
        self.labels: list[str] = []

    async def close(self) -> None:
        if self.labels:
            self.labels = []
        await self.repo.delete_user(self.poster_id)


class Comment:
    def __init__(self, poster_id: int, body: str) -> None:
        self.poster_id = poster_id
        self.body = body


def show(issue: Issue, comment) -> None:
    print(issue.poster_id, issue.title, issue.audit)
    print(comment.poster_id, comment.body)


def tally(comments) -> int:
    total: int = 0
    for comment in comments:
        total += comment.total
    return total
