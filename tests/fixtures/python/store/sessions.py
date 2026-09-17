class Session:
    @classmethod
    def start(cls) -> "Session":
        return cls()

    def close(self) -> None:
        pass


class Pool:
    @classmethod
    def start(cls) -> "Pool":
        return cls()


def open_session() -> Session:
    return Session.start()
