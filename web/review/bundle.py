#!/usr/bin/env python3
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile

USAGE = """usage: web/review/bundle.py MERL WASM OUT.json

Builds the four review repos (repos.sh), records the git calls of MERL --review on each
(record.py), and writes OUT.json for the site: per language the quest's steps, the files of
the branch and the replay table. Then check.mjs plays the steps on WASM, fills the answers
steps.json leaves to it, and fails on a step that does not end where it wants."""

HERE = pathlib.Path(__file__).resolve().parent


def fill(x, names):
    if isinstance(x, str):
        return re.sub(r"\{(\w+)\}", lambda m: names.get(m.group(1), m.group(0)), x)
    if isinstance(x, list):
        return [fill(i, names) for i in x]
    if isinstance(x, dict):
        return {k: fill(v, names) for k, v in x.items()}
    return x


def main():
    if len(sys.argv) != 4:
        sys.exit(USAGE)
    merl, wasm, out = sys.argv[1:]
    spec = json.loads((HERE / "steps.json").read_text())
    bundle = {k: v for k, v in spec.items() if k not in ("langs", "steps")}
    bundle["langs"] = {}
    with tempfile.TemporaryDirectory() as tmp:
        subprocess.run([HERE / "repos.sh", tmp], check=True)
        for lang, names in spec["langs"].items():
            repo = pathlib.Path(tmp, f"ratelimit-{lang}", spec["slug"])
            replay = pathlib.Path(tmp, f"{lang}.replay")
            r = subprocess.run([sys.executable, HERE / "record.py", merl, repo, spec["base"], replay,
                                *["c"] * 16], capture_output=True, text=True)
            print(lang, r.stdout.splitlines()[0] if r.returncode == 0 else r.stderr[-400:])
            if r.returncode != 0:
                sys.exit(1)
            listed = subprocess.run(["git", "-C", repo, "ls-files"], capture_output=True,
                                    text=True, check=True).stdout.splitlines()
            bundle["langs"][lang] = {
                "steps": fill(spec["steps"], names),
                "files": {f: (repo / f).read_text() for f in listed},
                "replay": replay.read_bytes().decode(),
            }
    pathlib.Path(out).write_text(json.dumps(bundle))
    subprocess.run(["node", HERE / "check.mjs", wasm, out, "--write"], check=True)
    print(out, os.path.getsize(out))


main()
