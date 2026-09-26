from app.repos import OrderRepo


class UnitOfWork:
    """One transaction: `with uow:` commits on success and rolls back on an error."""

    def __init__(self, db):
        self.db = db
        self.orders = OrderRepo(db)

    def __enter__(self):
        self.db.begin()
        return self

    def __exit__(self, exc_type, exc, tb):
        if exc_type is None:
            self.db.commit()
        else:
            self.db.rollback()
        return False
