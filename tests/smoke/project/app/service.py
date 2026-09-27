from app.config import Settings
from app.models import Order, OrderStatus
from app.repos import OrderRepo
from app.uow import UnitOfWork


class OrderNotFound(Exception):
    pass


class OrderService:
    def __init__(self, repo: OrderRepo, uow: UnitOfWork, settings: Settings):
        self.repo = repo
        self.uow = uow
        self.settings = settings

    def get(self, order_id: int) -> Order:
        order = self.repo.get(order_id)
        if order is None:
            raise OrderNotFound(order_id)
        return order

    def mark_paid(self, order_id: int) -> Order:
        order = self.get(order_id)
        order.status = OrderStatus.PAID
        self.repo.save(order)
        return order

    def cancel(self, order_id: int) -> Order:
        order = self.get(order_id)
        if not order.can_cancel():
            raise ValueError(f"order {order_id} is {order.status.value}")
        order.status = OrderStatus.CANCELLED
        with self.uow:
            self.uow.orders.save(order)
        return order
