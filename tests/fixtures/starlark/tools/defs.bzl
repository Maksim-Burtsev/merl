"""Macros and rules of the shop.

def weigh_parcel(grams):
Basket = provider(fields = ["items"])
"""

load(":providers.bzl", "TariffInfo")
#        ^ d: tools/providers.bzl:1
#                       ^ d: none

def shop_binary(name, srcs, **kwargs):
    native.genrule(name = name + "_gen", srcs = srcs, outs = [name + ".out"], cmd = "cp $< $@")
#   ^ d: none
#          ^ d: none
#                          ^ d: tools/defs.bzl:11
#                          status: name: local
#                                                                                     ^ d: none

SHOP_VISIBILITY = ["//visibility:public"]
#                     ^ d: none

def _tariff_impl(ctx):
    return [TariffInfo(rate = ctx.attr.rate)]
#           ^ d: tools/providers.bzl:1
#           status: TariffInfo: via import tools/providers.bzl
#                      ^ d: none

tariff = rule(implementation = _tariff_impl, attrs = {"rate": attr.int()})
#                               ^ d: tools/defs.bzl:22
#                                                      ^ d: none
tariff_aspect = aspect(implementation = _tariff_impl)
courier_repo = repository_rule(implementation = _tariff_impl)
couriers = module_extension(implementation = _tariff_impl)
weigh_transition = transition(implementation = _tariff_impl, inputs = [], outputs = [])

def weigh_parcel(grams):
    return grams * 2

def basket(name):
    weigh_parcel(name = name)
#   ^ d: tools/defs.bzl:36
    tariff_aspect
#   ^ d: tools/defs.bzl:31
    courier_repo
#   ^ d: tools/defs.bzl:32
    weigh_transition
#   ^ d: tools/defs.bzl:34
    couriers
#   ^ d: tools/defs.bzl:33

def courier(name, rate):
    native.filegroup(name = name, srcs = rate)
#                    ^ d: none
#                                        ^ d: tools/defs.bzl:51
