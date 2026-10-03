load(
    "//tools:defs.bzl",
    "tariff",
    weigh = "weigh_parcel",
#   ^ d: tools/defs.bzl:36
)

def courier_fee(tariff, rate = 2):
    return tariff(weigh(rate))
#          ^ d: cart/extra.bzl:8
#          status: tariff: local
#                 ^ d: tools/defs.bzl:36
#                 status: weigh_parcel: via import tools/defs.bzl

"""Wraps "a (rule" here."""

COURIER_RATE = 3
# ^ d: cart/extra.bzl:17
