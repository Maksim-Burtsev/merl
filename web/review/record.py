#!/usr/bin/env python3
import os
import pathlib
import re
import shlex
import subprocess
import sys
import tempfile
import time

USAGE = """usage: web/review/record.py MERL REPO BASE OUT [KEY...]

Runs MERL --review --base BASE in REPO inside tmux with GIT_TRACE, presses each KEY (a tmux
send-keys word, `w:SECS` waits), runs every git command it traced again, adds the per-file
variants the browser may ask for, and writes the replay table the browser build reads as
.git/merl-replay: per command its arguments joined by \\x1f, then `STATUS LEN`, then LEN bytes
of stdout. HOME is a fresh temporary directory, so no state of the machine's merl is read or
written."""


def run_merl(merl, repo, base, keys, home, trace):
    session = f"merl-record-{os.getpid()}"
    env = ["env", f"GIT_TRACE={trace}", f"HOME={home}", "XDG_STATE_HOME=", "XDG_CONFIG_HOME="]
    subprocess.run(["tmux", "new-session", "-d", "-s", session, "-x", "150", "-y", "40",
                    "-c", repo, *env, merl, "--review", "--base", base], check=True)
    time.sleep(1.5)
    for k in keys:
        if k.startswith("w:"):
            time.sleep(float(k[2:]))
            continue
        subprocess.run(["tmux", "send-keys", "-t", session, k], check=True)
        time.sleep(0.35)
    time.sleep(1)
    screen = subprocess.run(["tmux", "capture-pane", "-p", "-t", session],
                            capture_output=True, text=True).stdout
    subprocess.run(["tmux", "send-keys", "-t", session, "q"])
    time.sleep(0.4)
    subprocess.run(["tmux", "kill-session", "-t", session], capture_output=True)
    return screen


def traced_calls(trace, repo):
    roots = [repo, repo.replace("/private/tmp/", "/tmp/", 1)]

    def relative(a):
        for r in roots:
            if a.startswith(r + "/"):
                return a[len(r) + 1:]
        return a

    calls = []
    for line in pathlib.Path(trace).read_text().splitlines():
        m = re.search(r"trace: built-in: git (.*)$", line)
        if m:
            args = [relative(a) for a in shlex.split(m.group(1))]
            if args not in calls:
                calls.append(args)
    return calls


def main():
    if len(sys.argv) < 5:
        sys.exit(USAGE)
    merl, repo, base, out, *keys = sys.argv[1:]
    merl, repo = os.path.abspath(merl), os.path.realpath(repo)
    with tempfile.TemporaryDirectory() as tmp:
        trace = os.path.join(tmp, "trace")
        home = os.path.join(tmp, "home")
        os.mkdir(home)
        screen = run_merl(merl, repo, base, keys, home, trace)
        calls = traced_calls(trace, repo)

    def git(args):
        p = subprocess.run(["git", "-C", repo, *args], capture_output=True, stdin=subprocess.DEVNULL)
        return p.returncode, p.stdout

    merge_base = git(["merge-base", base, "HEAD"])[1].decode().strip()
    every = git(["ls-files"])[1].decode().splitlines()
    base_files = git(["ls-tree", "-r", "--name-only", merge_base])[1].decode().splitlines()
    extra = []
    for c in calls:
        if c[:1] == ["diff"] and "--" in c and "-U0" in c:
            i = c.index("--")
            extra += [c[:i + 1] + [f] for f in every]
    for f in base_files:
        extra += [["show", f"{merge_base}:{f}"], ["cat-file", "blob", f"{merge_base}:{f}"]]
    table = {}
    for c in calls + extra:
        k = "\x1f".join(c)
        if k not in table:
            table[k] = git(c)
    with open(out, "wb") as o:
        for k, (rc, stdout) in table.items():
            o.write(k.encode() + b"\n" + f"{rc} {len(stdout)}\n".encode() + stdout + b"\n")
    print(f"{len(calls)} traced, {len(table)} entries -> {out}")
    for c in calls:
        print("  git", " ".join(c)[:140])
    print(screen)


main()
