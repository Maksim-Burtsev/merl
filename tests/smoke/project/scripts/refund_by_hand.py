"""Refund an order by hand until the service can do it: python scripts/refund_by_hand.py ID"""
import sys

from app.config import load
from app.models import OrderStatus

ORDER_ID = int(sys.argv[1])
print(f"UPDATE orders SET status = '{OrderStatus.REFUNDED.value}' WHERE id = {ORDER_ID};")
print(f"-- then refund it in the payment provider's dashboard ({load().database_url})")
