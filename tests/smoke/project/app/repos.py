from app.models import Order, OrderStatus


class OrderRepo:
    """Orders in Postgres. Every method runs in the caller's transaction."""

    def __init__(self, db):
        self.db = db

    def get(self, order_id: int) -> Order | None:
        row = self.db.fetch_one("SELECT * FROM orders WHERE id = %s", order_id)
        return Order(**row) if row else None

    def find_by_customer(self, customer_id: int) -> list[Order]:
        rows = self.db.fetch_all(
            "SELECT * FROM orders WHERE customer_id = %s ORDER BY id", customer_id
        )
        return [Order(**row) for row in rows]

    def save(self, order: Order) -> None:
        self.db.execute(
            "INSERT INTO orders (id, customer_id, total, status) VALUES (%s, %s, %s, %s) "
            "ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status",
            order.id,
            order.customer_id,
            order.total,
            order.status.value,
        )

    def delete(self, order_id: int) -> None:
        self.db.execute("DELETE FROM orders WHERE id = %s", order_id)


class InMemoryOrderRepo:
    """Orders in a dict, for running the API without Postgres. Same methods as OrderRepo."""

    def __init__(self):
        self.orders: dict[int, Order] = {}

    def get(self, order_id: int) -> Order | None:
        return self.orders.get(order_id)

    def find_by_customer(self, customer_id: int) -> list[Order]:
        return sorted(
            (o for o in self.orders.values() if o.customer_id == customer_id),
            key=lambda o: o.id,
        )

    def find_by_status(self, status: OrderStatus) -> list[Order]:
        return [o for o in self.orders.values() if o.status is status]

    def save(self, order: Order) -> None:
        self.orders[order.id] = order

    def delete(self, order_id: int) -> None:
        self.orders.pop(order_id, None)

    def clear(self) -> None:
        self.orders.clear()
