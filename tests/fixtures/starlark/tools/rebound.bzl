load(":defs.bzl", "weigh_parcel")

GRAMS = 1

def own(weigh_parcel): return weigh_parcel
#                             ^ d: tools/rebound.bzl:5

def preset(GRAMS = 2): return GRAMS
#                             ^ d: tools/rebound.bzl:8

def after(): weigh_parcel = 1; return weigh_parcel
#                                     ^ d: tools/rebound.bzl:11

def unused(): f = lambda weigh_parcel: 0; return weigh_parcel
#                                                ^ d: tools/defs.bzl:36

def default(a = weigh_parcel): return a
#               ^ d: tools/defs.bzl:36

def called(xs):
    return weigh_parcel(1) + (lambda weigh_parcel: weigh_parcel)(2)
#          ^ d: tools/defs.bzl:36
#                                                  ^ d: tools/rebound.bzl:21

def listed(xs):
    return [weigh_parcel for weigh_parcel in xs] + [weigh_parcel(1)]
#           ^ d: tools/rebound.bzl:26
#                                                   ^ d: tools/defs.bzl:36

def keyed(xs):
    return {GRAMS: 1 for GRAMS in xs} | {GRAMS: 2}
#           ^ d: tools/rebound.bzl:31
#                                        ^ d: tools/rebound.bzl:3

def iterated(xs):
    return [weigh_parcel for weigh_parcel in weigh_parcel(xs)]
#                                            ^ d: tools/defs.bzl:36

def passed(xs):
    return f(lambda weigh_parcel: 1, weigh_parcel)
#                                    ^ d: tools/defs.bzl:36

[weigh_parcel(name) for name in ["a"]]
#             ^ d: tools/rebound.bzl:43

name = "x"
