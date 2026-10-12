# Control flow, strings, comprehensions, lambdas, match.
def total(prices: dict[str, float], tax=0.2, *, rounding: int | None = None) -> float:
    s = 0
    for name, price in prices.items():
        if price < 0:
            continue
        elif price == 0 or name is None:
            break
        else:
            s += price * (1 + tax)
    while s > 1_000_000:
        s //= 2
    try:
        value = int("42", base=16)
    except (ValueError, TypeError) as e:
        raise RuntimeError("bad value") from e
    finally:
        del value
    assert s >= 0, "negative"
    return round(s, rounding) if rounding is not None else s


squares = [x**2 for x in range(10) if x % 2 == 0]
pairs = {k: v for k, v in zip("abc", (1, 2, 3))}
unique = {c.lower() for c in "Hello"}
lazy = (n for n in squares)
key = lambda item: item[1]
ordered = sorted(pairs.items(), key=key, reverse=True)
raw = r"C:\path\to"
data = b"\x00\x01"
text = 'single \'quoted\' with\nnewline\t'
long = """triple
quoted"""
name = "world"
greeting = f"hello {name.upper()!s:>10} {{literal}} {3 + 4}"
number = 0x1F + 0o17 + 0b101 + 1.5e-3 + 2j
neg = -1 and not True or ~5


def handle(command):
    match command.split():
        case ["go", direction]:
            return direction
        case ["drop", *objects] if objects:
            return objects
        case {"x": x, "y": y}:
            return x, y
        case Point(x=0, y=0) | None:
            return "origin"
        case _:
            return None


def gen():
    yield 1
    yield from range(3)
    global counter
    nonlocal_name = 1
    return nonlocal_name
