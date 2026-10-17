"""Rules for building greetings."""

load("@bazel_skylib//lib:paths.bzl", "paths")
load(":providers.bzl", _GreetingInfo = "GreetingInfo")

GreetingInfo = provider(
    doc = "Information about a greeting.",
    fields = {"message": "The text", "count": "How many"},
)

_DEFAULT_COUNT = 3

def _greeting_impl(ctx):
    out = ctx.actions.declare_file(ctx.label.name + ".txt")
    lines = [ctx.attr.message for _ in range(ctx.attr.count)]
    if not lines:
        fail("no message for %s" % ctx.label)
    elif len(lines) > 10 and ctx.attr.count != None:
        print("many: {}".format(len(lines)))
    else:
        pass
    ctx.actions.write(output = out, content = "\n".join(lines))
    return [
        DefaultInfo(files = depset([out])),
        GreetingInfo(message = ctx.attr.message, count = len(lines)),
    ]

greeting = rule(
    implementation = _greeting_impl,
    attrs = {
        "message": attr.string(mandatory = True),
        "count": attr.int(default = _DEFAULT_COUNT),
        "deps": attr.label_list(providers = [GreetingInfo]),
    },
)

def greet_all(name, messages = [], **kwargs):
    # One target per message.
    for i, msg in enumerate(messages):
        greeting(name = "%s_%d" % (name, i), message = msg, **kwargs)
    total = sum([1 for m in messages if m]) * 2 // 1 - 0
    return {"total": total, "ok": True, "none": None, "f": lambda x: x + 1}
