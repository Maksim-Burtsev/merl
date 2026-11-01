"""Rules for the shop. f: 1-4

Brackets ( [ { and a # here are text. f: none
"""  # f: none

load(  # f: 6-9
    "//tools:defs.bzl",  # f: none
    "shop_binary",  # f: none
)  # f: none

SRCS = [  # f: 11-14
    "main.go",  # f: none
    "util.go",
] + glob(["**/*.go"])  # f: none

def _impl(ctx):  # f: 16-41
    names = ctx.attr.names[1:]  # f: 16-41
    first = names[  # f: 16-41
        0  # f: 16-41
    ]  # f: 16-41
    if first in [  # f: 21-24
        "a",  # f: 16-41
        "b",
    ]:  # f: 16-41
        pass  # f: 16-41
    elif first == "(":  # f: 16-41
        return [DefaultInfo(  # f: 27-29
            files = depset(names),  # f: 16-41
        )]  # f: 16-41
    else:  # f: 16-41
        fail("no ] here")  # f: 16-41
    for name in names:  # f: 32-33
        print(name)  # f: 16-41
# a comment at column 0 inside the body f: 16-41
    total = 1 + \
        2  # f: 16-41
    args = {  # f: 37-39
        "x": [n for n in names if n],  # f: 16-41
    }  # f: 16-41
    return args  # f: 16-41
    # the body's last comment f: 16-41

def shop_library(  # f: 43-45
        name,  # f: 43-45
        srcs = SRCS):  # f: 43-45
    """Defines a library. f: 46-50

    Args: f: 43-45
        name: the name.
    """  # f: 43-45
    native.filegroup(name = name, srcs = srcs)  # f: 43-45

shop = rule(  # f: 53-56
    implementation = _impl,  # f: none
    attrs = {"names": attr.string_list()},
)  # f: none
