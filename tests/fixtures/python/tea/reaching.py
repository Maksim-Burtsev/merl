def counted(backend):
    ids = backend.search("a")
    assert len(ids) == 1, (
    #          ^ d: tea/reaching.py:2
        f"got {len(ids)}"
    )
    ids = backend.search("b")
    return ids
    #      ^ d: tea/reaching.py:7


def rendered(tempdir):
    page = tempdir / "a"
    with open(tempdir) as f:
        page = tempdir / "b"
        page.write_text(f.read())
        #^ d: tea/reaching.py:15


def branched(flag):
    cup = 1
    if flag:
        cup = 2
    return cup
    #      ^ d: picker tea/reaching.py:21, tea/reaching.py:23


def one_line(flag):
    jar = 1
    jar = 2
    if flag: jar = 3
    return jar
    #      ^ d: !jump


def looped():
    tin = 1
    for _ in range(3):
        print(tin)
        #     ^ d: picker tea/reaching.py:37, tea/reaching.py:41
        tin = 2


def caught():
    bowl = 0
    try:
        bowl = 1
    except ValueError:
        return bowl
        #      ^ d: picker tea/reaching.py:45, tea/reaching.py:47


def held(ctx):
    vat = 1
    vat = 2
    with ctx() as vat:
        pass
    return vat
    #      ^ d: !jump


def failed():
    urn = 1
    urn = 2
    try:
        pass
    except ValueError as urn:
        pass
    return urn
    #      ^ d: !jump


def deleted():
    pan = 1
    pan = 2
    del pan
    return pan
    #      ^ d: !jump


def bumped():
    dish = 1
    dish = 2
    dish += 1
    return dish
    #      ^ d: !jump


def walrus():
    crock = 1
    crock = 2
    if (crock := 3):
        pass
    return crock
    #      ^ d: !jump


def closed():
    flask = 1
    flask = 2

    def bump():
        nonlocal flask
        flask = 3

    bump()
    return flask
    #      ^ d: !jump


def wrapped():
    mug = 1
    mug = 2
    total = (
        mug
        #^ d: !jump
    )
    return total


def comprehended():
    pot = 1
    pot = [2]
    return [pot
    #       ^ d: !jump
            for pot in range(3)]


kettle = 1
kettle = 2
print(kettle)
#     ^ d: !jump



def deferred():
    lid = 1
    lid = 2
    def peek(): return lid
    #                  ^ d: !jump
    lid = 3
    return peek()


def lazy(rows):
    cork = 1
    cork = 2
    later = (cork + r for r in rows)
    #        ^ d: !jump
    cork = 3
    return list(later)
