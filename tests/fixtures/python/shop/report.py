"""No import binds these names here: `d` searches by name, and literals and comments hold no
declaration."""


def summary(total: int) -> int:
    return discount(total) + len(Tariff.__name__)
    #      ^ d: shop/pricing.py:28
    #      status: by name, 1 match
    #                            ^ d: shop/pricing.py:12


def heavy(grams: int) -> int:
    return weigh(grams)
    #      ^ d: shop/warehouse.py:21


def capped() -> int:
    return RATE_CAP
    #      ^ d: shop/pricing.py:9
