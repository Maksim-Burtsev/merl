"""A bare name reaches a `def` at module level or in a function around it, never a method (#522)."""


class Relayer:
    def hand_over(self):
        return self

    async def hand_back(self):
        return self


def start():
    def hand_over():
        return 1

    return hand_over()
    #      ^ d: shop/reach.py:13


def stop():
    return hand_over()
    #      ^ d: picker shop/reach.py:5, shop/reach.py:13


def hand_back(steps):
    return steps


def finish(steps):
    return hand_back(steps)
    #      ^ d: shop/reach.py:25
