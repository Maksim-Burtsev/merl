from .sessions import Session, open_session

__all__ = ["Session", "connect", "open_session"]


def connect() -> Session:
    return open_session()
