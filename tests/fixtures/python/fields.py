from json import JSONEncoder

from repos import AuditLog, UserRepository


class Base:
    def __init__(self) -> None:
        self.audit = AuditLog()

    def summary(self) -> str:
        return ""


class Issue(Base):
    title: str
    poster_id: int = 0
    summary: str = ""

    def __init__(self, repo: UserRepository) -> None:
        super().__init__()
        self.repo = repo
        self.labels: list[str] = []

    async def close(self) -> None:
        if self.labels:
            self.labels = []
        self.audit = None
        await self.repo.delete_user(self.poster_id)


class Comment:
    """A comment on an issue.

    Attributes
    ----------
    body : str
    """

    def __init__(self, poster_id: int, body: str) -> None:
        self.poster_id = poster_id
        self.body = body


def show(issue: Issue, comment) -> None:
    print(issue.poster_id, issue.title, issue.audit, issue.summary)
    print(comment.poster_id, comment.body)


def tally(comments) -> int:
    total: int = 0
    for comment in comments:
        total += comment.total
    return total


class Encoder(JSONEncoder):
    class Options:
        indent = 2

    def default(self, o):
        if self.poster_id:
            return self.Options.indent
        return o


class Point:
    def __init__(self) -> None:
        self.x, self.offset = 0, 0

    def move(self) -> None:
        self.offset = 5


class Cursor:
    def __init__(self) -> None:
        self.offset = 0


def place(p) -> None:
    print(p.offset)
