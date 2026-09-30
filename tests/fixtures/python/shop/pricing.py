"""Prices.

A docstring that reads like code declares nothing:

class Tariff:
    def rate(self) -> int: ...
"""

RATE_CAP = 100


class Tariff:
    def rate(self) -> int:
        return 1

    def describe(self) -> str:
        return "tariff"


class Coupon:
    def rate(self) -> int:
        return 2

    def describe(self) -> str:
        return "coupon"


def discount(total: int) -> int:
    return min(total - 1, RATE_CAP)


async def settle(total: int) -> int:
    return total
