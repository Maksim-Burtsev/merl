#!/usr/bin/env python3
"""Records the reference of the fold bench: what Neovim folds with nvim-treesitter's `folds.scm`
in the projects of tools/d-bench/projects.tsv. Once, on a recording machine; never in CI.

    tools/fold-bench/install-parsers.sh DIR       # nvim-treesitter, tree-sitter CLI, parsers
    tools/fold-bench/record.py --parsers DIR [--lang go,rust] [--cache DIR]

For every file of a language it opens the file in headless Neovim (nvim 0.11+, `--clean`, no
injections), lets treesitter fold it and, on every line where a fold starts, closes the fold as
`zc` would and writes the line and the last line it hides to folds/LANG.tsv. A file whose tree
has a syntax error is left out. A language draws from the projects LANGS names; one with no
project of its own (yaml, json, markdown, …) from every project of projects.tsv. FOLD_ONLY holds
the projects of the languages the `d` bench has none for, kept out of projects.tsv because
tools/d-bench/run reads cursors for every project there. A language's files are taken in the order of the hash of their path until FOLDS folds are recorded.
"""
import argparse, hashlib, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
FOLDS = 6000
EVERY = None
FOLD_ONLY = {
    "plug": ["plug", "elixir", "https://github.com/elixir-plug/plug.git", "73404f851852a00ffb2014be95d4598900fa77b8", "-"],
    "rbs": ["rbs", "rbs", "https://github.com/ruby/rbs.git", "c78451b753cc6de113be8d096d09418d8170e9c6", "-"],
    "ohmyzsh": ["ohmyzsh", "zsh", "https://github.com/ohmyzsh/ohmyzsh.git", "60c9a7a839b790cd905d0fd4419435124fd1bdc0", "-"],
}
LANGS = {
    "go": (["caddy"], {"go": "go"}),
    "rust": (["ripgrep"], {"rs": "rust"}),
    "ts": (["outline"], {"ts": "typescript", "tsx": "tsx"}),
    "js": (["eslint"], {"js": "javascript", "mjs": "javascript", "cjs": "javascript", "jsx": "javascript"}),
    "c": (["redis"], {"c": "c", "h": "c"}),
    "cpp": (["leveldb"], {"cc": "cpp", "h": "cpp"}),
    "php": (["koel"], {"php": "php"}),
    "swift": (["Alamofire"], {"swift": "swift"}),
    "java": (["halo"], {"java": "java"}),
    "kotlin": (["nowinandroid"], {"kt": "kotlin", "kts": "kotlin"}),
    "csharp": (["eShop"], {"cs": "c_sharp"}),
    "ruby": (["mastodon"], {"rb": "ruby", "rake": "ruby"}),
    "objc": (["SDWebImage"], {"m": "objc", "h": "objc"}),
    "elixir": (["plug"], {"ex": "elixir", "exs": "elixir"}),
    "rbs": (["rbs"], {"rbs": "rbs"}),
    "zsh": (["ohmyzsh"], {"zsh": "zsh"}),
    "shell": (EVERY, {"sh": "bash", "bash": "bash"}),
    "lua": (EVERY, {"lua": "lua"}),
    "yaml": (EVERY, {"yml": "yaml", "yaml": "yaml"}),
    "json": (EVERY, {"json": "json"}),
    "toml": (EVERY, {"toml": "toml"}),
    "html": (EVERY, {"html": "html", "htm": "html"}),
    "css": (EVERY, {"css": "css", "scss": "scss"}),
    "markdown": (EVERY, {"md": "markdown"}),
}


def projects():
    rows = open(os.path.join(ROOT, "tools/d-bench/projects.tsv")).read().splitlines()
    return {**{r.split("\t")[0]: r.split("\t") for r in rows if not r.startswith("#")}, **FOLD_ONLY}


def ensure(cache, name, url, commit):
    d = os.path.join(cache, name)
    if not os.path.isdir(d):
        subprocess.run(["git", "clone", "-q", url, d], check=True)
    head = subprocess.run(["git", "-C", d, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    if head != commit:
        subprocess.run(["git", "-C", d, "fetch", "-q", "--depth", "1", "origin", commit], check=False)
        subprocess.run(["git", "-C", d, "checkout", "-q", commit], check=True)
    return d


def files(cache, lang):
    names, exts = LANGS[lang]
    every = projects()
    out = []
    for name in names or [n for n in every if n not in FOLD_ONLY]:
        _, _, url, commit, _ = every[name]
        d = ensure(cache, name, url, commit)
        for f in subprocess.run(["git", "-C", d, "ls-files"], capture_output=True, text=True).stdout.splitlines():
            ext = f.rsplit(".", 1)[-1] if "." in os.path.basename(f) else ""
            if ext in exts and os.path.isfile(os.path.join(d, f)):
                out.append((f"{name}/{f}", exts[ext]))
    return sorted(out, key=lambda p: hashlib.sha1(p[0].encode()).hexdigest())


def record(cache, parsers, lang):
    nvim = os.environ.get("NVIM", "nvim")
    rtp = f"set rtp^={parsers}/rt,{parsers}/nvim-treesitter/runtime"
    rows, total, errors = [], 0, 0
    tmp = tempfile.mkdtemp(prefix="fold-record-")
    pending = files(cache, lang)
    while pending and total < FOLDS:
        batch, pending = pending[:20], pending[20:]
        by_parser = {}
        for rel, parser in batch:
            by_parser.setdefault(parser, []).append(rel)
        got = {}
        for parser, rels in by_parser.items():
            listing, out = os.path.join(tmp, "list"), os.path.join(tmp, "out")
            open(listing, "w").write("".join(os.path.join(cache, r) + "\n" for r in rels))
            subprocess.run([nvim, "--clean", "--headless", "--cmd", rtp, "-l",
                            os.path.join(HERE, "folds.lua"), parser, listing, out], check=True)
            for line in open(out).read().splitlines():
                path, folds = line.split("\t")
                got[os.path.relpath(path, cache)] = folds
        for rel, _ in batch:
            if got[rel] == "error":
                errors += 1
            elif got[rel] and total < FOLDS:
                rows.append(f"{rel}\t{got[rel]}")
                total += len(got[rel].split())
    ts = subprocess.run(["git", "-C", f"{parsers}/nvim-treesitter", "rev-parse", "--short", "HEAD"],
                        capture_output=True, text=True).stdout.strip()
    version = subprocess.run([nvim, "--version"], capture_output=True, text=True).stdout.split("\n")[0]
    head = (f"# {version}, nvim-treesitter {ts}: {total} folds in {len(rows)} files, "
            f"{errors} files left out for a syntax error\n"
            "# path (project/file)\teach fold as line:last hidden line, 1-based\n")
    open(os.path.join(HERE, "folds", f"{lang}.tsv"), "w").write(head + "".join(r + "\n" for r in sorted(rows)))
    print(f"{lang}: {total} folds in {len(rows)} files, {errors} left out", file=sys.stderr)


def main():
    if sys.argv[1:] == ["parsers"]:
        print(",".join(sorted({p for _, exts in LANGS.values() for p in exts.values()} | {"markdown_inline"})))
        return
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--parsers", required=True)
    ap.add_argument("--lang", default=",".join(LANGS))
    ap.add_argument("--cache", default=os.environ.get("D_BENCH_CACHE", os.path.expanduser("~/.cache/merl-d-bench")))
    a = ap.parse_args()
    for lang in a.lang.split(","):
        record(os.path.realpath(a.cache), os.path.realpath(a.parsers), lang)


if __name__ == "__main__":
    main()
