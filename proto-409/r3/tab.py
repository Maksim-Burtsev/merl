#!/usr/bin/env python3
"""Types `merl ARGS<Tab>` in a real shell in tmux and reports what the shell made of it.

    tab.py KIND 'merl -r 1234' [...]      one line per query: the command line after Tab
    tab.py KIND --exec 'merl -r 1234'     also presses Enter and prints what merl received
"""
import subprocess, sys, time
import os
SOCK = ["tmux", "-L", os.environ.get("SOCK", "merl409r3")]
HERE = __file__.rsplit("/", 1)[0]


def t(*a, check=True):
    return subprocess.run(SOCK + list(a), capture_output=True, text=True, check=check).stdout


def screen():
    return t("capture-pane", "-p", "-t", "w")


def settle(timeout=4.0):
    last, same, end = None, 0, time.time() + timeout
    while time.time() < end:
        s = screen()
        same = same + 1 if s == last else 0
        if same >= 3:
            return s
        last = s
        time.sleep(0.08)
    return last


def start(kind, width=200, height=50):
    t("kill-session", "-t", "w", check=False)
    t("new-session", "-d", "-s", "w", "-x", str(width), "-y", str(height), f"{HERE}/shells.sh {kind}")
    for _ in range(100):
        s = screen()
        if "shop" in s:
            break
        time.sleep(0.1)
    settle()
    if kind == "zsh-real":
        t("send-keys", "-t", "w", "-l", "source <(COMPLETE=zsh /tmp/409r3/bin/merl); PATH=/tmp/409r3/bin:$PATH; clear")
        t("send-keys", "-t", "w", "Enter")
        time.sleep(0.5)
        settle()


def query(q, execute=False, tabs=1):
    t("send-keys", "-t", "w", "C-c")
    time.sleep(0.1)
    t("send-keys", "-t", "w", "C-l")
    settle()
    t("send-keys", "-t", "w", "-l", q)
    settle()
    for _ in range(tabs):
        t("send-keys", "-t", "w", "Tab")
        time.sleep(0.15)
        after = settle()
    got = None
    if execute:
        t("send-keys", "-t", "w", "Enter")
        for _ in range(60):
            s = screen()
            if "merl got:" in s and " END" in s:
                got = "".join(s.splitlines()).split("merl got:")[-1].split(" END")[0]
                got = "merl got:" + got + " END"
                break
            time.sleep(0.1)
    else:
        t("send-keys", "-t", "w", "C-c")
    settle()
    return after, got


if __name__ == "__main__":
    kind, args = sys.argv[1], sys.argv[2:]
    ex = "--exec" in args
    args = [a for a in args if a != "--exec"]
    start(kind)
    for q in args:
        after, got = query(q, ex)
        print(f"===== {kind}: {q!r}")
        print("\n".join(l for l in after.splitlines() if l.strip()))
        if ex:
            print("RECEIVED:", got)
    t("kill-server", check=False)
