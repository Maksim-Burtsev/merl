# A file with a syntax error in the middle.
def ok(a, b):
    return a + b


def broken(x:
    if x > 1
        return [x, (x + 1]
    print("still here")


class After:
    value = ok(1, 2)
