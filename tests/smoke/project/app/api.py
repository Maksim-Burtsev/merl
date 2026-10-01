import json
from http import HTTPStatus
from typing import cast

from app.service import OrderNotFound, OrderService


def order_json(order) -> str:
    return json.dumps(
        {"id": order.id, "status": order.status.value, "total": str(order.total)},
        sort_keys=True,
    )


def get_order(service: OrderService, order_id: int) -> tuple[int, str]:
    try:
        return HTTPStatus.OK, order_json(service.get(order_id))
    except OrderNotFound:
        return HTTPStatus.NOT_FOUND, json.dumps({"error": "not found"})


def list_orders(repo, customer_id: int) -> tuple[int, str]:
    orders = repo.find_by_customer(customer_id)
    return HTTPStatus.OK, json.dumps([order.id for order in orders])


def cancel_order(container, order_id: int) -> tuple[int, str]:
    service = cast(
        OrderService, container.get("orders")
    )
    return HTTPStatus.OK, order_json(service.cancel(order_id))


def page_links(total: int) -> list[str]:
    def page_url(n):
        return f"?page={n}"

    return [page_url(n) for n in range(1, total + 1)]
