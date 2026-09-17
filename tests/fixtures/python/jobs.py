import store.sessions as sessions
from repos import UserRepository
from store import connect, open_session
from store.sessions import Session as StoreSession


def nightly(repo: UserRepository) -> None:
    connect()
    open_session()
    sessions.open_session().close()
    StoreSession.start()
