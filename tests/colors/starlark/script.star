# A plain Starlark script.
def fib(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a

values = [fib(i) for i in range(10) if i % 2 == 0]
names = {"x": 1.5, "y": 0x10, "z": 1e3}
print("fib: %d, %s" % (values[-1], names["x"]))
assert_eq = lambda a, b: a == b or fail("not equal")
