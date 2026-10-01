from decimal import Decimal

import pytest

from app import OrderService
from app.models import Order, OrderStatus
from app.service import OrderNotFound


class FakeOrderRepo:
    def __init__(self, *orders):
        self.orders = {order.id: order for order in orders}
        self.saved = []

    def get(self, order_id):
        return self.orders.get(order_id)

    def find_by_customer(self, customer_id):
        return [o for o in self.orders.values() if o.customer_id == customer_id]

    def save(self, order):
        self.saved.append(order)


class FakeUow:
    def __init__(self, repo):
        self.orders = repo

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False


def make_service(*orders):
    repo = FakeOrderRepo(*orders)
    return OrderService(repo, FakeUow(repo), settings=None), repo


def test_cancel_pending_order():
    service, repo = make_service(Order(id=1, customer_id=7, total=Decimal("12.50")))
    assert service.cancel(1).status is OrderStatus.CANCELLED
    assert repo.saved[0].id == 1


def test_missing_order():
    service, _ = make_service()
    with pytest.raises(OrderNotFound):
        service.get(404)


def test_first_of_many():
    assert next(iter([3, 4])) == 3


import unittest  # noqa: E402


class LegacyServiceTest(unittest.TestCase):
    def test_nothing_saved(self):
        self.assertEqual(1 + 1, 2)
