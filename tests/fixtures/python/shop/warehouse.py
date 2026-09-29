BANNER = r"""
def discount(total):
class Basket:
"""


class Courier:
    def __init__(self, name: str) -> None:
        self.name = name

    def next(self) -> "Courier":
        return self

    def _fmt(self) -> str:
        return self.name


# def weigh(): a comment that reads like code declares nothing either


def weigh(grams: int) -> int:
    return grams // 1000
