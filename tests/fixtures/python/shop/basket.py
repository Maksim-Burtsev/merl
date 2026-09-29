from shop.pricing import Tariff, discount
#     ^ d: shop/__init__.py:1
#          ^ d: shop/pricing.py:1
from shop import pricing
from shop.warehouse import Courier as Carrier
import shop.warehouse as wh
import shop

from .pricing import Coupon

WEIGHT_LIMIT: int = 30


class Basket:
    items: list[str]
    owner = "guest"

    def __init__(self, tariff: Tariff) -> None:
        #                      ^ d: shop/pricing.py:12
        self.tariff = tariff
        self.coupon = Coupon()
        #             ^ d: shop/pricing.py:20

    def gross(self) -> int:
        return discount(self.tariff.rate())
        #      ^ d: shop/pricing.py:28
        #                    ^ d: shop/basket.py:20
        #                           ^ d: picker shop/pricing.py:13, shop/pricing.py:21; want shop/pricing.py:13 (#453)

    def bonus(self) -> int:
        return self.coupon.rate() + self.gross()
        #                  ^ d: shop/pricing.py:21
        #                                ^ d: shop/basket.py:24

    def label(self) -> str:
        def _fmt(text: str) -> str:
            return text.upper()

        return _fmt(self.owner)
        #      ^ d: picker shop/basket.py:36, shop/warehouse.py:14; want shop/basket.py:36 (#338)
        #                ^ d: shop/basket.py:16


class GiftBasket(Basket):
    def gross(self) -> int:
        return super().gross() + len(self.items)
        #              ^ d: shop/basket.py:24
        #                                 ^ d: shop/basket.py:15


def describe_any(offer) -> str:
    return offer.describe()
    #            ^ d: picker shop/pricing.py:16, shop/pricing.py:24


async def pay(total: int) -> int:
    return await pricing.settle(total)
    #                    ^ d: shop/pricing.py:32


def dispatch() -> str:
    courier = Carrier("post")
    #         ^ d: shop/warehouse.py:7
    other = wh.Courier("van")
    #          ^ d: shop/warehouse.py:7
    return courier.name + other.name
    #              ^ d: shop/warehouse.py:9


def restock(discount: int) -> int:
    return discount + WEIGHT_LIMIT
    #      ^ d: shop/basket.py:70
    #                 ^ d: shop/basket.py:11


def overweight(grams: int) -> bool:
    WEIGHT_LIMIT = 50
    return wh.weigh(grams) > WEIGHT_LIMIT
    #         ^ d: shop/warehouse.py:21
    #                        ^ d: shop/basket.py:77


def from_package() -> int:
    return shop.Tariff().rate()
    #           ^ d: shop/pricing.py:12


def ship(
    weight: int,
    courier: Carrier,
) -> int:
    return weight + len(courier.name)
    #      ^ d: shop/basket.py:88; want shop/basket.py:89 (#338)


def first(couriers: list[Carrier]) -> Carrier:
    return next(iter(couriers))
    #      ^ d: shop/warehouse.py:11; want none (#336)


def checkout(basket: Basket) -> int:
    return basket.gross() + Basket(tariff=Tariff()).bonus()
    #             ^ d: shop/basket.py:24
    #                                  ^ d: !jump; want shop/basket.py:18 (#316)
