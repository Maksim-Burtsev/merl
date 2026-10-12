import re

WORD = re.compile(r"(?P<word>[A-Za-z]+)\s*=\s*(\d{1,3}|none)")
SPLIT = re.split(r"[,;]\s*", "a, b; c")
MATCH = re.match("^start" "(?:middle)+end$", "x")


def describe(name, count):
    print("%s has %d items, %5.2f%% done" % (name, count, 0.5))
    return "%(name)s: %(count)03d" % {"name": name, "count": count}


def not_a_format(a, b):
    return "%s" + a % b
