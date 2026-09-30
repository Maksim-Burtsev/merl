"""A builtin's name the file binds keeps the lookup it had (#336)."""


def next(steps):
    return steps[0]


def advance(steps):
    return next(steps)
    #      ^ d: shop/stepper.py:4


def peek(steps):
    next = steps[-1]
    return next
    #      ^ d: shop/stepper.py:14


class Printer:
    def format(self, x):
        return str(x)

    render = format
    #        ^ d: shop/stepper.py:20
