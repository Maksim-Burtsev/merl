from typing import overload


@overload
def gauge(key: str) -> int | None: ...


@overload
def gauge(key: str, default: int) -> int: ...


def gauge(key: str, default: int | None = None) -> int | None:
    return default
