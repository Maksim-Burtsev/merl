load("//tools:defs.bzl", "tariff", courier_rule = "shop_binary")
#                                    ^ d: tools/defs.bzl:11

tariff(name = "rates", rate = 3)
#^ d: tools/defs.bzl:28
#                         ^ d: none

courier_rule(
# ^ d: tools/defs.bzl:11
    name = "courier",
    srcs = [
        "//api",
#          ^ d: api/BUILD.bazel:9
        "//api:api.go",
#               ^ d: api/api.go:1
        "@shop//api:api",
#          ^ d: api/BUILD.bazel:9
        "@//api",
#           ^ d: api/BUILD.bazel:9
        ":rates",
#          ^ d: cart/BUILD:4
        "rates",
#         ^ d: cart/BUILD:4
        "courier.go",
#          ^ d: cart/courier.go:1
        ":server_gen",
#          ^ d: none
        "@rules_go",
#           ^ d: MODULE.bazel:3
        "@bazel_gazelle",
#           ^ d: MODULE.bazel:4
    ],
    deps = select({
        "//api:api": [],
#           ^ d: none
    }),
)
