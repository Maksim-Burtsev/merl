def ration(cap):
    #      ^ d: picker shop/caps.py:1
    return cap


def rations(
    first,
    cap,
    # ^ d: picker shop/caps.py:1
):
    return first


ration(cap=1)
#      ^ d: shop/limits.py:1
#      status: cap: parameter of ration


later = lambda: ration(cap=2)
#                      ^ d: shop/limits.py:1
