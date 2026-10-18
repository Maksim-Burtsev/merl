#!/usr/bin/env python3
"""Dart for the d bench, outside the repo: sample cursors and ask `dart language-server`.

  dart_bench.py sample ROOT N OUT [--exclude d,d]
  dart_bench.py oracle ROOT CURSORS ANSWERS
"""
import importlib.machinery, importlib.util, os, sys

loader = importlib.machinery.SourceFileLoader(
    "rec", os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "record.py"))
rec = importlib.util.module_from_spec(importlib.util.spec_from_loader("rec", loader))
loader.exec_module(rec)
DART = os.path.join(rec.LSP, "dart-sdk", "bin", "dart")

rec.SPEC["dart"] = dict(exts=(".dart",), lc=("//",), bc=("/*", "*/"), triple=True,
                        decl={"final", "late", "mixin", "get", "set", "typedef", "library", "part"})
rec.KW["dart"] = """abstract as assert async await base break case catch class const continue
covariant default deferred do dynamic else enum export extends extension external factory false
final finally for Function get hide if implements import in interface is late library mixin new
null of on operator part required rethrow return sealed set show static super switch sync this
throw true try type typedef var void when while with yield Object String int double num bool List
Map Set Iterable Future Stream Duration DateTime Uri Exception Error Never Null Type Symbol print
identical override""".split()

orig_server = rec.server


def server(lang, root):
    if lang != "dart":
        return orig_server(lang, root)
    cache = os.path.join(rec.LSP, "dart-server-cache")
    return rec.Lsp([DART, "--suppress-analytics", "language-server", f"--cache={cache}",
                    "--client-id=merl-d-bench"], root,
                   env={"PUB_CACHE": os.path.join(rec.LSP, "dart-pub-cache")})


rec.server = server

if __name__ == "__main__":
    a = sys.argv[1:]
    root = os.path.realpath(a[1])
    if a[0] == "sample":
        ex = a[a.index("--exclude") + 1].split(",") if "--exclude" in a else ()
        rec.sample("dart", os.path.basename(root), root, int(a[2]), a[3], ex)
    else:
        rec.oracle("dart", root, a[2], a[3])
