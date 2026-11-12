from chains import T


def defaulted[T](x=T, y: T | None = None) -> T:
    #              ^ d: chains.py:5
    #                    ^ d: tea/generic_default.py:4
    return x
