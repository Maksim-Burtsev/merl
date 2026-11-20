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
    if first in [  # f: 21-31
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

def shop_library(  # f: 43-51
        name,  # f: 43-51
        srcs = SRCS):  # f: 43-51
    """Defines a library. f: 46-50

    Args: f: 43-51
        name: the name.
    """  # f: 43-51
    native.filegroup(name = name, srcs = srcs)  # f: 43-51

shop = rule(  # f: 53-56
    implementation = _impl,  # f: none
    attrs = {"names": attr.string_list()},
)  # f: none

def shop_suite(  # f: 58-66
        name,  # f: 58-66
):  # f: 58-66
    def shop_case(  # f: 61-65
            case,  # f: 61-65
            size = "small"):  # f: 61-65
        native.sh_test(name = case, size = size)  # f: 61-65
        return case  # f: 61-65
    return shop_case(name)  # f: 58-66

def shop_pick(names, cases):  # f: 68-90
    for name in [  # f: 69-81
        n  # f: 68-90
        for n in names  # f: 68-90
    ]:  # f: 68-90
        if (name in cases and  # f: 73-80
                cases[name]):  # f: 68-90
            print(name)  # f: 68-90
        elif (name == "a" or  # f: 76-77
              name == "b"):  # f: 68-90
            pass  # f: 68-90
        else:  # f: 68-90
            fail(name)  # f: 68-90
        # the for's last comment f: 68-90
    if (names and  # f: 82-88
            cases.get(  # f: 83-85
                names[0],  # f: 68-90
            )):  # f: 68-90
        pass  # f: 68-90
    elif names:  # f: 68-90
        pass  # f: 68-90
    for case in cases: print([  # f: 89-90
        case])  # f: 68-90
