#!/usr/bin/env python3
"""The smoke test: before a release, plays every scenario in tmux on the new build and on the last
release, and writes what differs to a report (.claude/skills/smoke-test/SKILL.md reads it); on
every PR (--golden, in CI), plays them on the new build alone against the screens checked in.

    tests/smoke/run.py                           build this checkout, fetch the last release, play all
    tests/smoke/run.py --golden                  every checkpoint's screen against tests/smoke/screens/
    tests/smoke/run.py --update --only edit      rewrite edit's screens after a change made on purpose
    tests/smoke/run.py --only review,edit        these two, or all whose name holds a part given
    tests/smoke/run.py --old target/release/merl the new build against itself
    tests/smoke/run.py --gif demo.gif --only python-d    one scenario on the new build, as a GIF
    tests/smoke/run.py --all-shots               each checkpoint that differs from the last release, a PNG each, in two themes
    tests/smoke/run.py --selftest                the runner's own failure paths, on fake binaries

Without --new the checkout is built (`cargo build --release --locked`, its own target/); without
--old the last GitHub release's macOS binary is downloaded once into ~/.cache/merl-smoke/TAG/,
checked against the release's sha256 and against `--version` before it is kept. A run rebuilds
the fixture (setup.sh) in WORK, /tmp/merl-smoke by default: a short path that is the same on
every run, so the two builds' screens can be compared. A lock on WORK makes a second --golden
or --gif run wait while one plays; it refuses a second run that writes a report (the first one's
would be deleted) or has a --work given. The report goes to
WORK/out, or WORK/out-only for --only, so a rerun of one scenario leaves the full report in
place; a directory --out names is deleted only if a run made it.

Each scenario is played on the new build, then the old one, from a fresh copy of the fixture in a
tmux pane of 185x55, a laptop's full screen, with an empty HOME. JOBS scenarios play at once, each
in its own tmux server and its own copy (WORK/run, WORK/run-1, ...); the ones with timed steps or
on a demo checkout play after them, alone, so their timings are taken on a quiet machine. A scenario is a steps file, one
step per line, `#` comments and blank lines skipped; the grammar of tools/cast.py and
assets/tapes/record.py plus a few verbs of its own:

    merl ARGS          start merl in the project; later in the file: quit it (it must exit 0)
                       and start it again, the same HOME kept
    wait TEXT          an assertion and a checkpoint: the pane shows TEXT within 10 s, and the
                       screen is saved once it has settled; a timeout is a FAIL, merl dead a CRASH.
                       At a wait the new build met, the old one waits 4x the new one's time, 2 s
                       at least (OLD_WAIT): a release lacking a feature fails its wait sooner
    key K*N            tmux key names, each N times (`key@P` paces a GIF, here keys go 10 ms apart)
    type TEXT          the characters: at once here, one keystroke each in a GIF
    sleep S            seconds, at most 0.3: the waits do the syncing
    run CMD            a shell command in the project, before the next step: the agent in the next
                       split; a non-zero exit fails the scenario. $SMOKE_TMUX is the tmux
                       command of merl's pane: `$SMOKE_TMUX show-buffer` holds what OSC 52 copied
    check CMD          a run that only reads what merl wrote (the bytes on disk, the pane, the
                       clipboard): a non-zero exit runs it again until it exits 0, for 10 s, as
                       a wait waits, so it never reads a file before merl has saved it (#1033)
    size WxH           the pane's size: before the first `merl`, the size it starts at
    time               the next wait is a timed step: three samples on each build, the fastest kept
    caption TEXT       skipped here; --gif draws it under the screen from then on, over the step
    show               skipped (record.py's)
    include FILE       the steps of FILE, relative to the repository
    project gitea      header: run in the README's gitea checkout, made by assets/tapes/setup.sh
                       (fetched once, 59 MB), instead of the fixture; `project polar` in its
                       polar checkout (an 80 MB fetch). assets/tapes/record.py reads the same line

--golden plays each scenario once, on the new build only, and compares each checkpoint with
tests/smoke/screens/SCENARIO/NN.txt: the wait's text, then the screen as text, trailing blanks cut,
then the cursor. Colours are left out. Beside them, keys.txt: the actions of `KEYS` merl counted a
press of in the play (keys.tsv in its HOME), which src/app/tests/smoke.rs reads (#310). It prints a
diff per screen that differs and exits 1 on any difference or failure. --update writes the screens
of the scenarios it played instead (only those that played through); commit them with the change,
so the PR's diff shows every screen it changes. RELEASE_ONLY names the scenarios CI does not play, and SCREEN_SKIPPED the checkpoints
whose screen depends on the machine (their wait is still checked).

--all-shots plays each scenario on the new build and on the last release (or --old), once in
merl's default theme and once in a light one (THEMES), and draws to OUT/THEME/SCENARIO/NN.png,
OUT being WORK/shots (WORK/shots-only for --only), every checkpoint whose screen, colours
included, differs from the old build's (old above new) or that the old build never reaches;
OUT/shots.md lists them and counts the ones left out as the same. The skill's agents look at each
PNG against its checklist: contrast, colour and alignment are only seen in a picture (#312), and a
screen the last release drew alike was looked at on its release (#707).

A scenario that is not about the tree hides it (`t`) after its first wait, so that a change to the
tree shows in one checkpoint per scenario, not in all of them. What a wait may name, and where a new
feature's steps go: AGENTS.md, its bullets on tests/smoke.
"""
import argparse, concurrent.futures, contextlib, datetime, difflib, fcntl, hashlib, io, json, os, platform, queue, re, shlex
import shutil, signal, subprocess, sys, tempfile, threading, time, traceback

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
HERE = os.path.join(ROOT, "tests", "smoke")
sys.path.insert(0, os.path.join(ROOT, "tools"))
sys.dont_write_bytecode = True
import cast
from PIL import Image, ImageDraw, ImageFont

CACHE = os.path.expanduser("~/.cache/merl-smoke")
WORK = "/tmp/merl-smoke"
DEMOS = ("gitea", "polar")
PACE, POLL, TIMEOUT, SLEEP_CAP, SIZE = 0.01, 0.02, 10.0, 0.3, (185, 55)
HOLD = 1.2
JOBS, OLD_WAIT = 4, (4, 2.0)
STOP = threading.Event()
MARK = ".merl-smoke-report"
SGR = re.compile(r"\x1b\[[0-9;]*m")
VOLATILE = re.compile(r"(?<=merl-tutor-)\d+")
AGENT = {"GIT_CONFIG_GLOBAL": "/dev/null", "GIT_AUTHOR_NAME": "agent", "GIT_COMMITTER_NAME": "agent",
         "GIT_AUTHOR_EMAIL": "agent@example.com", "GIT_COMMITTER_EMAIL": "agent@example.com",
         "GIT_AUTHOR_DATE": "2026-09-03T10:00:00Z", "GIT_COMMITTER_DATE": "2026-09-03T10:00:00Z"}
RUST_HOMES = {k: p for k, p in ((k, os.environ.get(k) or os.path.expanduser(d))
                                 for k, d in (("RUSTUP_HOME", "~/.rustup"), ("CARGO_HOME", "~/.cargo")))
              if os.path.isdir(p)}
SCREENS = os.path.join(HERE, "screens")
RELEASE_ONLY = {
    "scale": "gitea's checkout, a 59 MB fetch, and its timed steps mean nothing on a shared runner",
    "readme-review": "polar's checkout, an 80 MB fetch; the README's own recording plays it too",
}
SCREEN_SKIPPED = {
    ("python-d", "json/__init__.py"): "the standard library of the machine's Python",
    ("python-d", "cancel: by name"): "the cancel methods of the machine's Python standard library",
    ("python-d", "assertEqual → TestCase.assertEqual (via self: LegacyServiceTest)"):
        "the unittest of the machine's Python standard library",
    ("go-d", "Errorf: via import fmt"): "the standard library of the machine's Go",
    ("go-d", "Done → Context.Done (via ctx: Context)"): "the standard library of the machine's Go",
    ("c-d", "printf: by name"): "the system headers of the machine",
    ("review", "30 days: 2 sessions"): "merl --reviews prints today's date and the time each session took",
    ("review", "merl exited: 0"): "merl --reviews prints today's date and the time each session took",
}
LEGEND = [("PASS", "every checkpoint the same on both builds"),
          ("DIFF", "the new build played it through; the differences below each need a verdict"),
          ("FAIL", "a wait timed out: the text never showed"),
          ("CRASH", "merl died: a panic (101) or a signal, named"),
          ("EXIT", "merl exited with another status, on its own or on q"),
          ("HUNG", "merl did not quit on q"),
          ("RUN", "a `run` or `check` step failed: wrong bytes on disk, or the agent's command"),
          ("ERROR", "the runner failed, not merl: rerun, fix run.py if it comes back")]
THEMES = ((None, None), ("github-light", "GitHub Light Default"))
NEVER_SHOWN = "(never shown)"
CLOCK_AT = int(datetime.datetime(2026, 10, 12, 14, 32, tzinfo=datetime.timezone.utc).timestamp())
DEFAULT_ONLY = {"theme": "its first wait is a theme the picker lists around the default one"}


class Merl(cast.Pane):

    def __init__(self, binary, home, size, err, slot=0):
        self.sock, self.home, self.binary, self.err = f"smoke{os.getpid()}{suffix(slot)}", home, binary, err
        self.size = size
        self.socket = os.path.join(os.environ.get("TMUX_TMPDIR") or "/tmp", f"tmux-{os.getuid()}", self.sock)

    def start(self, args, project):
        options = [("remain-on-exit", "on"), ("status", "off"), ("default-shell", "/bin/sh"),
                   ("remain-on-exit-format", "merl exited: #{pane_dead_status}")]
        self.tmux("start-server", *[w for o in options for w in (";", "set", "-g", *o)],
                  ";", "set", "-s", "set-clipboard", "on",
                  ";", "new-session", "-d", "-x", str(self.size[0]), "-y", str(self.size[1]), "-c", project,
                  self.command(args))

    def tmux(self, *args):
        if STOP.is_set():
            raise RuntimeError("the run was stopped")
        out = subprocess.run(["tmux", "-L", self.sock, "-f", "/dev/null", *args],
                             capture_output=True, text=True)
        if out.returncode:
            raise RuntimeError("tmux %s: %s" % (args[0], out.stderr.strip()))
        return out.stdout

    def command(self, args):
        unset = " ".join("-u " + v for v in os.environ if v.startswith("XDG_"))
        homes = " ".join(f"{k}={shlex.quote(p)}" for k, p in RUST_HOMES.items())
        return "exec env %s HOME=%s %s COLORTERM=truecolor GOTOOLCHAIN=local TZ=UTC MERL_CLOCK_AT=%d %s 2>>%s" % (
            unset, shlex.quote(self.home), homes, CLOCK_AT, shlex.join([os.path.abspath(self.binary)] + args),
            shlex.quote(self.err))

    def restart(self, args):
        self.tmux("respawn-pane", "-k", "-t", "0", self.command(args))

    def dead(self):
        f = "#{pane_dead},#{pane_dead_status},#{pane_dead_signal}"
        dead, status, sig = self.tmux("display", "-p", "-t", "0", f).strip().split(",")
        if dead != "1":
            return None
        if sig:
            return 128 + (int(sig) if sig.isdigit() else getattr(signal, "SIG" + sig.upper(), 0))
        return int(status or 0)

    def frame(self):
        capture, cursor = super().frame()
        return mask(capture), cursor

    def settled(self):
        last, end = self.frame(), time.monotonic() + 1
        while time.monotonic() < end:
            time.sleep(0.05)
            if (now := self.frame()) == last:
                break
            last = now
        return last

    def quit(self):
        for keys in (["Escape", "Escape", "q"], ["q"]):
            if self.dead() is None:
                self.tmux("send-keys", "-t", "0", *keys)
                end = time.monotonic() + 1.5
                while self.dead() is None and time.monotonic() < end:
                    time.sleep(POLL)
        return self.dead()

    def close(self):
        tmux = ["tmux", "-L", self.sock, "-f", "/dev/null"]
        subprocess.run([*tmux, "kill-server"], capture_output=True)
        end = time.monotonic() + 5
        while subprocess.run([*tmux, "ls"], capture_output=True).returncode == 0 and time.monotonic() < end:
            time.sleep(POLL)
        if os.path.exists(self.socket):
            os.unlink(self.socket)


def died(status):
    if status is None:
        return "HUNG"
    if status >= 128:
        try:
            return f"CRASH {signal.Signals(status - 128).name}"
        except ValueError:
            return f"CRASH signal {status - 128}"
    return f"CRASH {status}" if status == 101 else f"EXIT {status}"


def load(path):
    project, steps = None, []
    label = os.path.relpath(path, ROOT)
    label = os.path.basename(path) if label.startswith("..") else label
    for n, raw in enumerate(open(path), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        verb, _, arg = line.partition(" ")
        verb, _, pace = verb.partition("@")
        if verb == "include":
            steps += load(os.path.join(ROOT, arg))[1]
        elif verb == "project":
            project = arg
        else:
            steps.append((f"{label}:{n}", verb, pace, arg))
    first = next(s for s in steps if s[1] not in ("size", "caption"))
    assert first[1] == "merl", f"{path}: the first step is `merl [ARGS]`"
    return project, steps


def suffix(slot):
    return f"-{slot}" if slot else ""


def servers_left(pid):
    return [slot for slot in range(JOBS)
            if not subprocess.run(["tmux", "-L", f"smoke{pid}{suffix(slot)}", "ls"], capture_output=True).returncode]


def fresh(work, project, slot=0):
    run = os.path.join(work, "run" + suffix(slot))
    shutil.rmtree(run, ignore_errors=True)
    shutil.copytree(os.path.join(work, "base"), run, symlinks=True)
    if project in DEMOS:
        demo = os.path.join(work, "demo", project)
        subprocess.run('git checkout -q -- . && git clean -fdq && rm -f "$(git rev-parse --git-common-dir)/merl/viewed" "$(git rev-parse --git-dir)/merl/recent"',
                       shell=True, cwd=demo, check=True)
        return demo
    return os.path.join(run, "orders")


class Recorder:

    def __init__(self):
        self.caption, self.step, self.shots, self.stop, self.thread = "", "", [], threading.Event(), None

    def start(self, pane):
        def sample():
            while not self.stop.is_set():
                now = time.monotonic()
                self.shots.append((now, pane.frame(), pane.size, (self.caption, self.step)))
                self.stop.wait(max(0.0, 0.1 - (time.monotonic() - now)))
        self.thread = threading.Thread(target=sample, daemon=True)
        self.thread.start()

    def end(self):
        self.stop.set()
        if self.thread:
            self.thread.join()


def play(binary, path, work, timed_only=False, rec=None, theme=None, slot=0, met=None):
    run = os.path.join(work, "run" + suffix(slot))
    home, err = os.path.join(run, "home"), os.path.join(run, "stderr")
    r = {"checkpoints": [], "times": {}, "slow": [], "status": "ok", "timed": False, "met": []}
    size, pane, timed, t0 = list(SIZE), None, False, time.monotonic()
    last_key, step, runner, seen, gone = t0, "", "", None, False

    def keep(label, frame):
        r.setdefault("last", [label, *frame, size])

    def keep_dead():
        if seen:
            r.setdefault("last", [f"the last checkpoint before merl died: {seen[0]}", *seen[1:]])
        else:
            keep("the screen after merl died", pane.frame())
    try:
        project, steps = load(path)
        cwd = fresh(work, project, slot)
        open(err, "w").close()
        if theme:
            os.makedirs(os.path.join(home, ".config", "merl"))
            open(os.path.join(home, ".config", "merl", "config.toml"), "w").write(f'theme = "{theme}"\n')
        last_timed = max((i for i, s in enumerate(steps) if s[1] == "time"), default=-1)
        r["timed"] = last_timed >= 0
        for i, (where, verb, pace, arg) in enumerate(steps):
            step = f"{where} `{verb} {arg}`"
            if rec and not rec.thread and pane and verb not in ("wait", "sleep", "caption"):
                rec.start(pane)
            if pane and not gone and verb in ("merl", "key", "type", "run", "check") and (s := pane.dead()) is not None:
                r["status"] = f"{died(s)} before {step}"
                keep_dead()
                break
            if verb == "merl":
                if pane is None:
                    pane = Merl(binary, home, size, err, slot)
                    pane.start(shlex.split(arg), cwd)
                else:
                    before = pane.frame()
                    if (s := pane.quit()) != 0:
                        r["status"] = f"{died(s)} on the restart at {step}"
                        keep("before the restart", before)
                        break
                    pane.restart(shlex.split(arg))
                    seen, gone = None, False
                last_key = time.monotonic()
            elif verb == "size":
                size = [int(v) for v in arg.split("x")]
                if pane:
                    pane.tmux("resize-window", "-t", "0", "-x", str(size[0]), "-y", str(size[1]))
                    pane.size = size
                    last_key = time.monotonic()
            elif verb == "wait":
                ms, dead_at, k = None, None, len(r["met"])
                limit = min(TIMEOUT, max(OLD_WAIT[1], OLD_WAIT[0] * met[k] / 1000)) if k < len(met or []) else TIMEOUT
                while time.monotonic() - last_key < limit:
                    if arg in pane.tmux("capture-pane", "-p", "-t", "0"):
                        ms = round((time.monotonic() - last_key) * 1000)
                        break
                    if pane.dead() is not None:
                        dead_at = dead_at or time.monotonic()
                        if time.monotonic() - dead_at > 0.5:
                            break
                    time.sleep(POLL)
                if ms is None:
                    s = pane.dead()
                    r["status"] = f"{'FAIL' if s is None else died(s)} at {step}"
                    r["checkpoints"].append([f"{where} wait {arg} {NEVER_SHOWN}", *pane.frame(), size])
                    if s is None:
                        keep(f"{where} wait {arg}: the screen when it gave up", pane.frame())
                    else:
                        keep_dead()
                    break
                r["met"].append(ms)
                if timed:
                    r["times"][f"{where} wait {arg}"], timed = ms, False
                elif ms > 1000:
                    r["slow"].append([f"{where} wait {arg}", ms])
                r["checkpoints"].append([f"{where} wait {arg}", *pane.settled(), size])
                seen, gone = r["checkpoints"][-1], pane.dead() is not None
                if timed_only and i > last_timed:
                    break
                if rec:
                    rec.step = f"wait {arg}  ✓ {ms} ms"
                    time.sleep(HOLD)
            elif verb in ("key", "type"):
                keys = (list(arg) if rec else [arg]) if verb == "type" else [
                    k.partition("*")[0] for k in arg.split()
                    for _ in range(int(k.partition("*")[2] or 1))]
                gap = float(pace or (0.12 if verb == "type" else 0.25)) if rec else PACE
                if rec:
                    rec.step = f"{verb} {arg}"
                for k in keys:
                    pane.type(k) if verb == "type" else pane.tmux("send-keys", "-t", "0", k)
                    time.sleep(gap)
                last_key = time.monotonic()
            elif verb == "sleep":
                time.sleep(float(arg) if rec else min(float(arg), SLEEP_CAP))
            elif verb in ("run", "check"):
                if rec:
                    rec.step = f"{verb} {arg[:90]}"
                env = {**os.environ, **AGENT, **RUST_HOMES, "HOME": home,
                       "SMOKE_TMUX": f"tmux -L {pane.sock} -f /dev/null"}
                end = time.monotonic() + (TIMEOUT if verb == "check" else 0)
                while (out := subprocess.run(arg, shell=True, cwd=cwd, env=env, capture_output=True,
                                             text=True)).returncode and time.monotonic() < end:
                    if not gone and (s := pane.dead()) is not None:
                        r["status"] = f"{died(s)} at {step}"
                        keep_dead()
                        break
                    time.sleep(POLL)
                if r["status"] != "ok":
                    break
                if out.returncode:
                    said = (out.stdout + out.stderr).strip()[-300:]
                    r["status"] = f"RUN {out.returncode} at {step}: {said}"
                    keep(f"{where} run: the screen when it failed", pane.frame())
                    break
                last_key = time.monotonic()
            elif verb == "time":
                timed = True
            elif verb == "caption":
                if rec:
                    rec.caption = arg
            elif verb != "show":
                raise ValueError(f"unknown step `{verb}`")
    except Exception as e:
        r["status"] = f"ERROR at {step}: {e}"
        runner = traceback.format_exc()
        if pane:
            try:
                keep("the screen when the runner failed", pane.frame())
            except Exception:
                if seen:
                    r.setdefault("last", [f"the last checkpoint before the runner failed: {seen[0]}", *seen[1:]])
    finally:
        if rec:
            rec.end()
        if pane:
            try:
                if r["status"] == "ok" and not gone and (s := pane.dead()) is not None:
                    r["status"] = f"{died(s)} after its last step"
                    keep_dead()
                before = pane.frame()
                if (s := pane.quit()) != 0 and r["status"] == "ok":
                    r["status"] = f"{died(s)} on q"
                    keep("before q", before)
            except Exception:
                runner += traceback.format_exc()
                r["status"] = r["status"] if r["status"] != "ok" else "ERROR on q"
            finally:
                pane.close()
    r["seconds"] = round(time.monotonic() - t0, 1)
    r["keys"] = pressed(home)
    r["stderr"] = ((open(err).read() if os.path.exists(err) else "") + runner)[-2000:]
    return r


def pressed(home):
    path = os.path.join(home, ".local", "state", "merl", "keys.tsv")
    rows = [line.rstrip("\n").split("\t") for line in open(path)] if os.path.exists(path) else []
    return sorted({r[1] for r in rows if len(r) > 2 and r[2] != "0"})


def scenario(path, new, old, work, slot=0):
    res = {"new": play(new, path, work, slot=slot)}
    met = {"new": None, "old": res["new"]["met"]}
    res["old"] = play(old, path, work, slot=slot, met=met["old"])
    if res["new"]["timed"]:
        for side in res:
            res[side]["samples"] = {k: [v] for k, v in res[side]["times"].items()}
        for n, order in enumerate((("old", "new"), ("new", "old")), 2):
            for side in order:
                again = play(new if side == "new" else old, path, work, timed_only=True, slot=slot, met=met[side])
                for k, v in again["times"].items():
                    res[side]["samples"].setdefault(k, []).append(v)
                res[side]["stderr"] += again["stderr"]
                if again["status"] != "ok" and res[side]["status"] == "ok":
                    res[side]["status"] = f"{again['status']} (play {n} of 3, for the timed steps)"
                    res[side]["last"] = again.get("last")
    return res


def alone(path):
    project, steps = load(path)
    return project in DEMOS or any(s[1] == "time" for s in steps)


def pooled(picked, fn):
    later = [n for n, p in picked.items() if alone(p)]
    slots, pool = queue.Queue(), concurrent.futures.ThreadPoolExecutor(JOBS)
    for slot in range(JOBS):
        slots.put(slot)

    def job(name):
        slot = slots.get()
        try:
            return fn(name, slot)
        finally:
            slots.put(slot)
    try:
        futures = {n: pool.submit(job, n) for n in picked if n not in later}
        for name, future in futures.items():
            yield name, future.result()
        for name in later:
            yield name, fn(name, 0)
    except BaseException:
        STOP.set()
        raise
    finally:
        pool.shutdown(cancel_futures=True)


def hexed(v):
    return "#%02x%02x%02x" % v if isinstance(v, tuple) else str(v)


def boxes(old, new):
    size = new[3]
    g = [cast.parse(c[1], *size, cast.XTERM) for c in (old, new)]
    todo = {(x, y) for y in range(size[1]) for x in range(size[0]) if g[0][y][x] != g[1][y][x]}
    out, rows = [], set()
    while todo:
        stack, box = [todo.pop()], []
        while stack:
            x, y = stack.pop()
            box.append((x, y))
            near = {(x + dx, y + dy) for dx in range(-3, 4) for dy in (-1, 0, 1)} & todo
            todo -= near
            stack += near
        (x0, x1), (y0, y1) = [(min(v), max(v)) for v in zip(*box)]
        text = ["".join(c[0] for c in g[i][y]) for i in (0, 1) for y in range(y0, y1 + 1)]
        for _ in range(24):
            x0 -= x0 > 0 and all(t[x0 - 1] not in " │┌└" for t in text)
            x1 += x1 < size[0] - 1 and all(t[x1 + 1] not in " │┐┘" for t in text)
        crop = [["".join(c[0] for c in g[i][y][x0:x1 + 1]).rstrip() for y in range(y0, y1 + 1)]
                for i in (0, 1)]
        rows |= set(range(y0, y1 + 1))
        where = f"rows {y0}-{y1}, cols {x0}-{x1}"
        if crop[0] == crop[1]:
            attrs = ("fg", "bg", "bold", "italic")
            changed = sorted({f"{a} {hexed(g[0][y][x][k])}→{hexed(g[1][y][x][k])}" for x, y in box
                              for k, a in enumerate(attrs, 1) if g[0][y][x][k] != g[1][y][x][k]})
            key = "colour: " + ", ".join(changed)
            shown = ", ".join(changed[:4]) + f", and {len(changed) - 4} more" * (len(changed) > 4)
            out.append((key, f"colour: {shown}, over `{crop[1][0].strip()[:50]}`", where))
        else:
            d = [line for line in difflib.unified_diff(*crop, n=0, lineterm="")
                 if not line.startswith(("@@", "---", "+++"))]
            d = d[:12] + [f"… {len(d) - 12} more"] * (len(d) > 12)
            what = "text:\n```diff\n" + "\n".join(d) + "\n```"
            out.append((what, what, where))
    if old[2] != new[2]:
        what = f"cursor (x,y,shown) {old[2]} → {new[2]}"
        out.append((what, what, ""))
        rows |= {int(old[2].split(",")[1]), int(new[2].split(",")[1])}
    return out, sorted(r for r in rows if r < size[1])


def fonts_cell(px):
    fonts = [ImageFont.truetype(cast.FONT, px, index=i) for i in range(4)]
    return fonts, (round(fonts[0].getlength("M")), sum(fonts[0].getmetrics()))


def png_pair(pair, rows, png, fonts, cell):
    band, gap = cell[1] + 8, 6
    size = [max(v) for v in zip(*(c[3] for _, c in pair))]
    h = size[1] * cell[1]
    img = Image.new("RGB", (size[0] * cell[0] + gap, len(pair) * (h + band)), (50, 50, 70))
    draw = ImageDraw.Draw(img)
    for i, (name, c) in enumerate(pair):
        x, y, shown = (int(v) for v in c[2].split(","))
        top = i * (h + band) + band
        draw.text((gap + 4, top - band + 4), f"{name}  {c[0]}", font=fonts[1], fill=(255, 220, 120))
        screen = cast.render(cast.parse(c[1], *c[3]), (x, y) if shown else None, fonts, cell)
        img.paste(screen, (gap, top))
        for y in rows:
            draw.rectangle([0, top + y * cell[1], gap - 2, top + (y + 1) * cell[1] - 1], fill=(255, 80, 80))
    img.save(png)


def unreleased(names):
    text = open(os.path.join(ROOT, "CHANGELOG.md")).read()
    part = text.split("## [Unreleased]", 1)[1].split("\n## [", 1)[0]
    changes = os.path.join(ROOT, "changes")
    for name in sorted(os.listdir(changes) if os.path.isdir(changes) else [], key=lambda n: (len(n), n)):
        part += "\n" + open(os.path.join(changes, name)).read()
    cited = {n: set(re.findall(r"#(\d+)", open(p).read())) for n, p in names.items()}
    out = []
    for entry in re.split(r"\n- ", "\n" + part)[1:]:
        entry = " ".join(line.strip() for line in entry.splitlines() if not line.startswith("#")).strip()
        issues = re.findall(r"#(\d+)", entry.rsplit("(", 1)[-1])
        out.append((entry[:90] + "…" * (len(entry) > 90), issues,
                    [n for n, c in cited.items() if c & set(issues)]))
    return out


def report(results, out, head, names):
    fonts, cell = fonts_cell(14)
    rows, found, timing, slow, failed_plays = [], {}, [], [], []
    for name, r in results.items():
        new, old = r["new"], r["old"]
        kinds = []
        for i, (o, n) in enumerate(zip(old["checkpoints"], new["checkpoints"]), 1):
            if o[1:] == n[1:]:
                kinds.append("same")
                continue
            failed = NEVER_SHOWN in o[0] + n[0]
            colour = SGR.sub("", o[1]) == SGR.sub("", n[1]) and o[2:] == n[2:]
            kinds.append("failed" if failed else "colour" if colour else "text")
            what, diff_rows = boxes(o, n)
            png_pair([("old", o), ("new", n)], diff_rows, os.path.join(out, name, f"{i:02d}.png"), fonts, cell)
            for key, shown, where in what * (not failed):
                first = "; ".join(filter(None, (where, f"{name}/{i:02d}.png")))
                found.setdefault(key, [shown, first, {}])[2].setdefault(name, []).append(i)
        kinds += ["missing"] * abs(len(new["checkpoints"]) - len(old["checkpoints"]))
        bad = len(kinds) - kinds.count("same")
        verdict = new["status"].split()[0] if new["status"] != "ok" else f"DIFF {bad}" if bad else "PASS"
        counts = ", ".join(f"{kinds.count(k)} {k}" for k in ("same", "text", "colour", "failed", "missing")
                           if k in kinds)
        rows.append(f"| {name} | **{verdict}** | {new['status']} | {old['status']} | {counts} | "
                    f"{new['seconds']} / {old['seconds']} |")
        for side in ("new", "old"):
            s = r[side]
            if s["status"] == "ok" and not s["stderr"].strip():
                continue
            line = f"- {name}, {side}: {s['status']}"
            if s.get("last"):
                png_pair([(side, s["last"])], [], os.path.join(out, name, f"{side}-last.png"), fonts, cell)
                line += f"; {s['last'][0]}: {name}/{side}-last.png"
            failed_plays.append(line)
            if s["stderr"].strip():
                failed_plays += ["  ```", *("  " + x for x in s["stderr"].strip().splitlines()), "  ```"]
        for label, samples in new.get("samples", {}).items():
            o = old.get("samples", {}).get(label, [])
            n_ms, o_ms = min(samples), min(o) if o else None
            flag = "**slower**" if o_ms is not None and n_ms > 2 * o_ms and n_ms - o_ms > 200 else ""
            timing.append(f"| {name} | `{label}` | {n_ms} | {'—' if o_ms is None else o_ms} | "
                          f"{' '.join(map(str, samples))} / {' '.join(map(str, o))} | {flag} |")
        for side in ("new", "old"):
            slow += [f"| {name} | {side} | `{label}` | {ms} |" for label, ms in r[side]["slow"]]
    detail = []
    widest = sorted(found.values(), key=lambda f: -sum(map(len, f[2].values())))
    for k, (shown, where, places) in enumerate(widest, 1):
        first, *body = shown.split("\n")
        at = "; ".join(f"{n} #{spans(sorted(set(i)))}" for n, i in places.items())
        detail += [f"{k}. {first} in {at} (first at {where})"] + ["   " + line for line in body]
    entries = [f"| {e} | {', '.join('#' + i for i in issues) or '—'} | {', '.join(s) or '—'} |"
               for e, issues, s in unreleased(names)]
    rep = [head, "", "| scenario | verdict | new | old | checkpoints | seconds new / old |",
           "|---|---|---|---|---|---|", *rows, "",
           "; ".join(f"**{v}** {meaning}" for v, meaning in LEGEND) + ".", "",
           "## Differences (old → new), each once, with every checkpoint it shows in", "",
           "Each differing checkpoint is `SCENARIO/NN.png` next to this report, old above new.", "",
           *(detail or ["none"]), "",
           "## Plays that did not end ok, and stderr", "", *(failed_plays or ["none"]), "",
           "## Timed steps (ms from the key to the text, the fastest of 3 on each build)", "",
           "The pane is polled about every 30 ms: a step under 50 ms reads as 5 one time, 40 the next.", "",
           "**slower**: over 2× and over 200 ms slower than the old build.", "",
           "| scenario | step | new | old | samples new / old | |", "|---|---|---|---|---|---|", *timing, "",
           "## Waits over 1 s (one sample)", "",
           *(["| scenario | build | wait | ms |", "|---|---|---|---|", *slow] if slow else ["none"]), "",
           "## Unreleased entries and the scenarios that cite their issues", "",
           "| entry | issues | scenarios |", "|---|---|---|", *entries]
    open(os.path.join(out, "report.md"), "w").write("\n".join(rep) + "\n")


def spans(nums):
    out, start = [], None
    for i, n in enumerate(nums):
        start = n if start is None else start
        if i + 1 == len(nums) or nums[i + 1] != n + 1:
            out.append(f"{start}–{n}" if n > start else str(n))
            start = None
    return ", ".join(out)


ESC = re.compile(r"\x1b(\[[0-9;:?]*[A-Za-z]|\][^\x07\x1b]*(\x07|\x1b\\))")


def screen(name, checkpoint):
    label, capture, cursor = checkpoint[:3]
    wait = label.split(" wait ", 1)[-1]
    head = [f"wait {wait}"]
    if why := SCREEN_SKIPPED.get((name, wait)):
        return "\n".join(head + [f"(screen not compared: {why})"]) + "\n"
    rows = [line.rstrip() for line in ESC.sub("", capture).split("\n")]
    while rows and not rows[-1]:
        rows.pop()
    return "\n".join(head + rows + [f"cursor (x,y,shown) {cursor}"]) + "\n"


def golden(new, picked, work, update, screens=SCREENS):
    good = True
    for name, r in pooled(picked, lambda name, slot: play(new, picked[name], work, slot=slot)):
        got ={f"{i:02d}.txt": screen(name, c) for i, c in enumerate(r["checkpoints"], 1)}
        if r["keys"]:
            got["keys.txt"] = "".join(k + "\n" for k in r["keys"])
        where = os.path.join(screens, name)
        if r["status"] != "ok":
            good = False
            print(f"{name:14} {r['status']}", flush=True)
            if r.get("last"):
                print(f"  {r['last'][0]}:\n" + screen(name, r["last"]))
            if r["stderr"].strip():
                print(r["stderr"].strip())
            continue
        if update:
            shutil.rmtree(where, ignore_errors=True)
            os.makedirs(where)
            for f, text in got.items():
                open(os.path.join(where, f), "w").write(text)
            print(f"{name:14} {len(got)} screens written, {r['seconds']} s", flush=True)
            continue
        have = sorted(os.listdir(where)) if os.path.isdir(where) else []
        diff = []
        for f in sorted(set(have) | set(got)):
            want = open(os.path.join(where, f)).read() if f in have else ""
            if want != got.get(f, ""):
                rel = os.path.relpath(os.path.join(where, f), ROOT)
                diff += difflib.unified_diff(want.splitlines(), got.get(f, "").splitlines(),
                                             f"{rel} (checked in)", f"{rel} (this build)", lineterm="")
        good &= not diff
        print(f"{name:14} {'DIFF' if diff else 'ok'}, {len(got)} screens, {r['seconds']} s", flush=True)
        print("\n".join(diff) + "\n" * bool(diff), flush=True)
    if not good and not update:
        print("A screen changed on purpose: tests/smoke/run.py --update --only NAME, and commit "
              "tests/smoke/screens/ with the change.")
    return good


def shots(new, old, tag, picked, work, out):
    report_dir(out)
    fonts, cell = fonts_cell(14)
    tag = tag or old
    lines, results, t0 = ["# merl smoke: the checkpoints that differ from " + tag, "",
                          f"`{new}` ({version(new)}) against {tag} ({version(old)})", "",
                          "| PNG | checkpoint | |", "|---|---|---|"], {}, time.monotonic()
    failed, dark, same = [], (cast.DEFAULT_FG, cast.DEFAULT_BG, cast.ANSI[:]), 0
    for theme, terminal in THEMES:
        cast.DEFAULT_FG, cast.DEFAULT_BG, cast.ANSI[:] = dark
        if terminal and os.path.exists(os.path.join(cast.GHOSTTY_THEMES, terminal)):
            cast.use_ghostty(terminal)
        label = theme or "default"
        for name, path in picked.items():
            if theme and name in DEFAULT_ONLY:
                failed.append(f"- {label}/{name}: not played, {DEFAULT_ONLY[name]}")
                continue
            r = results.setdefault(label, {})[name] = play(new, path, work, theme=theme)
            was = play(old, path, work, theme=theme)["checkpoints"]
            where = os.path.join(out, label, name)
            os.makedirs(where)
            for i, c in enumerate(r["checkpoints"], 1):
                o = was[i - 1] if i <= len(was) and NEVER_SHOWN not in was[i - 1][0] else None
                if o and o[1:] == c[1:]:
                    same += 1
                    continue
                pair = [(tag, o), (label, c)] if o else [(label, c)]
                png_pair(pair, [], os.path.join(where, f"{i:02d}.png"), fonts, cell)
                lines.append(f"| {label}/{name}/{i:02d}.png | `{c[0]}` | {'changed' if o else 'new'} |")
            if r["status"] != "ok":
                failed.append(f"- {label}/{name}: {r['status']}")
                if r.get("last"):
                    png_pair([(label, r["last"])], [], os.path.join(where, "last.png"), fonts, cell)
                    failed[-1] += f"; {r['last'][0]}: {label}/{name}/last.png"
            print(f"{label:13} {name:14} {r['status']}, {len(r['checkpoints'])} shots, {r['seconds']} s",
                  flush=True)
    count = sum(len(r["checkpoints"]) for t in results.values() for r in t.values())
    lines[4:4] = [f"{count} checkpoints: {count - same} PNGs, {same} the same as {tag} and not drawn; "
                  f"{time.monotonic() - t0:.0f} s. Plays that did not end ok, or were not played:",
                  "", *(failed or ["none"]), ""]
    cast.DEFAULT_FG, cast.DEFAULT_BG, cast.ANSI[:] = dark
    open(os.path.join(out, "shots.md"), "w").write("\n".join(lines) + "\n")
    print(os.path.join(out, "shots.md"))
    return results


def version(binary):
    try:
        out = subprocess.run([binary, "--version"], capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return out.stdout.strip() if out.returncode == 0 and out.stdout.startswith("merl") else None


def last_release(tag=None):
    gh = lambda *a: subprocess.run(["gh", *a], cwd=ROOT, capture_output=True, text=True)
    if not tag:
        out = gh("release", "view", "--json", "tagName", "-q", ".tagName")
        tag = out.stdout.strip() or sys.exit(f"the last release: gh says {out.stderr.strip()}")
    binary = os.path.join(CACHE, tag, "merl")
    if version(binary):
        return binary, tag
    arch = "aarch64" if platform.machine() == "arm64" else platform.machine()
    asset = f"merl-{arch}-apple-darwin"
    os.makedirs(CACHE, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=CACHE) as tmp:
        out = gh("release", "download", tag, "-p", f"{asset}.tar.gz", "-p", f"{asset}.sha256", "-D", tmp)
        if out.returncode:
            sys.exit(f"{tag}: gh release download failed: {out.stderr.strip()}")
        sums = os.path.join(tmp, f"{asset}.sha256")
        want = open(sums).read().split()[0] if os.path.exists(sums) else None
        got = hashlib.sha256(open(os.path.join(tmp, f"{asset}.tar.gz"), "rb").read()).hexdigest()
        if want and got != want:
            sys.exit(f"{tag}: {asset}.tar.gz has sha256 {got}, the release says {want}")
        subprocess.run(["tar", "xzf", f"{asset}.tar.gz"], cwd=tmp, check=True)
        if not version(os.path.join(tmp, "merl")):
            sys.exit(f"{tag}: the merl in {asset}.tar.gz does not answer --version")
        shutil.rmtree(os.path.dirname(binary), ignore_errors=True)
        os.makedirs(os.path.dirname(binary))
        os.replace(os.path.join(tmp, "merl"), binary)
    return binary, tag


def lock(work, wait=False):
    os.makedirs(work, exist_ok=True)
    f = open(os.path.join(work, "lock"), "a+")
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        f.seek(0)
        busy = f"{work} is in use by another smoke run: {f.read().strip() or 'unknown'}"
        if not wait:
            sys.exit(f"{busy}.\nWait for it to end, or pass another --work DIR.")
        print(f"{busy}; waiting for it to end", flush=True)
        fcntl.flock(f, fcntl.LOCK_EX)
    f.truncate(0)
    f.write(f"pid {os.getpid()}, {' '.join(sys.argv)}, from {os.getcwd()}, since {time.ctime()}\n")
    f.flush()
    return f


def unlock(f):
    f.truncate(0)
    f.close()


def report_dir(out):
    if os.path.isdir(out) and os.listdir(out):
        if not os.path.exists(os.path.join(out, MARK)):
            sys.exit(f"{out} holds files a smoke run did not write: pass another --out")
        shutil.rmtree(out)
    os.makedirs(out, exist_ok=True)
    open(os.path.join(out, MARK), "w").close()


def gif(binary, path, work, out):
    rec = Recorder()
    r = play(binary, path, work, rec=rec)
    print(r["status"])
    fonts, cell = fonts_cell(18)
    line_h = cell[1] + 4
    merged = []
    for k, (when, frame, size, band) in enumerate(rec.shots):
        end = rec.shots[k + 1][0] if k + 1 < len(rec.shots) else when + 0.1
        if merged and merged[-1][:3] == [frame, size, band]:
            merged[-1][3] += end - when
        else:
            merged.append([frame, size, band, end - when])
    images, durations = [], []
    for (capture, cursor), size, (caption, step), seconds in merged:
        x, y, shown = (int(v) for v in cursor.split(","))
        screen = cast.render(cast.parse(capture, *size), (x, y) if shown else None, fonts, cell)
        img = Image.new("RGB", (screen.width, screen.height + 2 * line_h + 16), (50, 50, 70))
        img.paste(screen, (0, 0))
        draw = ImageDraw.Draw(img)
        draw.text((14, screen.height + 8), caption, font=fonts[1], fill=(255, 220, 120))
        draw.text((14, screen.height + 8 + line_h), step, font=fonts[0], fill=(190, 190, 210))
        images.append(img)
        durations.append(max(30, round(seconds * 1000)))
    cast.save_gif(images, durations, out)


FAKE = """#!/bin/bash
trap 'printf "\\033[?1049l"' EXIT  # as merl does: its screen is gone once it has quit
%(s)s
printf '\\033[?1049h'; echo ready
while IFS= read -rsn1 c; do
  case "$c" in
    q) %(q)s ;;
    a) kill -ABRT $$ ;;
    e) exit 3 ;;
    d) %(d)s echo done ;;
    ';') printf ';' ;;
  esac
done
"""
DIE = ("run $SMOKE_TMUX send-keys -t 0 e; for _ in $(seq 500); do"
       " $SMOKE_TMUX display -p -t 0 '#{pane_dead}' | grep -q 1 && exit 0; sleep 0.02; done; exit 1\n")
SELFTEST = {
    "pass": ("merl\nwait ready\nkey d\nwait done\n", {}, {}, "PASS", ""),
    "signal": ("merl\nwait ready\nkey a\nwait done\n", {}, {}, "CRASH", "CRASH SIGABRT at"),
    "exit": ("merl\nwait ready\nkey e\nwait done\n", {}, {}, "EXIT", "EXIT 3 at"),
    "fail": ("merl\nwait ready\nwait never\n", {}, {}, "FAIL", "FAIL at fail.steps:3"),
    "hung": ("merl\nwait ready\n", {"q": ":"}, {}, "HUNG", "HUNG on q"),
    "run": ("merl\nwait ready\nrun test -e nowhere\n", {}, {}, "RUN", "RUN 1 at"),
    "run-once": ("merl\nwait ready\nrun test -e saved || ! touch saved\n", {}, {}, "RUN", "RUN 1 at"),
    "check": ("merl\nwait ready\ncheck test -e saved || ! touch saved\n", {}, {}, "PASS", ""),
    "check-dies": ("merl\nwait ready\nkey d\ncheck test -e nowhere\n", {"d": "sleep 0.2; kill -ABRT $$;"}, {},
                   "CRASH", "CRASH SIGABRT at"),
    "semicolon": ("merl\nwait ready\ntype x;;\nwait ;;\n", {}, {}, "PASS", ""),
    "end": ("merl\nwait ready\nsize 60x10\n" + DIE, {}, {}, "EXIT", "EXIT 3 after its last step"),
    "restart": ("merl\nwait ready\n" + DIE + "merl\nwait ready\n", {}, {}, "EXIT", "EXIT 3 before"),
    "quit": ("merl\nwait ready\nkey q\nwait merl exited: 0\nrun true\nmerl\nwait ready\n", {}, {}, "PASS", ""),
    "quit-3": ("merl\nwait ready\nkey e\nwait merl exited: 0\n", {}, {}, "EXIT", "EXIT 3 at"),
    "restarted": ("merl\nwait ready\nmerl\nwait ready\n",
                  {"s": '[ -e "$0.started" ] && exit 101; touch "$0.started"'}, {}, "CRASH", "CRASH 101 at"),
    "rerun": ("merl\nwait ready\nkey d\ntime\nwait done\n",
              {"d": '[ -e "$0.played" ] && exit 101; touch "$0.played";'}, {}, "CRASH",
              "(play 2 of 3, for the timed steps)"),
    "slower": ("merl\nwait ready\nkey d\ntime\nwait done\n", {"d": "sleep 0.5;"}, {}, "PASS", "**slower**"),
    "short": ("merl\nwait ready\nkey d\nwait done\n", {}, {"d": "continue;"}, "DIFF", "FAIL at short.steps:4"),
    "diff-red": ("merl\nwait ready\nkey d\nwait done\n", {"d": "printf '\\033[31m';"}, {}, "DIFF", "#cd0000"),
    "diff-blue": ("merl\nwait ready\nkey d\nwait done\n", {"d": "printf '\\033[34m';"}, {}, "DIFF", "#0000ee"),
    "error": ("merl\nwait ready\nrun $SMOKE_TMUX kill-server; for _ in $(seq 500); do"
              " $SMOKE_TMUX ls || exit 0; sleep 0.02; done; exit 1\nwait ready\n", {}, {}, "ERROR", "tmux capture-pane"),
}


def mask(capture):
    def line(text):
        pids = VOLATILE.findall(text)
        if not pids:
            return text
        start = VOLATILE.search(text).start()
        tail = VOLATILE.sub("NNNNN", text)[start:]
        pad = max(re.finditer(r" +", tail), key=lambda r: len(r.group()), default=None)
        width = len(pad.group()) + sum(len(p) - 5 for p in pids) if pad else 0
        if width > 0:
            tail = tail[:pad.start()] + " " * width + tail[pad.end():]
        return text[:start] + tail
    return "\n".join(map(line, capture.split("\n")))


def selftest():
    bar = lambda pid, pad: f"merl-tutor-{pid}/\n\x1b[1mmerl-tutor-{pid}/  [code]\x1b[0m{' ' * pad}14:32"
    want = bar("NNNNN", 20)
    for pid, pad in (("1234", 21), ("12345", 20), ("1234567", 18)):
        assert mask(bar(pid, pad)) == want, f"a pid of {len(pid)} digits: {mask(bar(pid, pad))!r}"
    tmp = tempfile.mkdtemp(prefix="smoke-selftest-")
    work, out = os.path.join(tmp, "work"), os.path.join(tmp, "out")
    for d in ("orders", "home"):
        os.makedirs(os.path.join(work, "base", d))
    names, results, bins = {}, {}, {}
    for name, (steps, new, old, _, _) in SELFTEST.items():
        names[name] = os.path.join(tmp, name + ".steps")
        open(names[name], "w").write(steps)
        bins[name] = [os.path.join(tmp, f"{name}-{side}") for side in ("new", "old")]
        for binary, acts in zip(bins[name], (new, old)):
            open(binary, "w").write(FAKE % {"q": "exit 0", "d": "", "s": "", **acts})
            os.chmod(binary, 0o755)
    for name, r in pooled(names, lambda name, slot: scenario(names[name], *bins[name], work, slot)):
        results[name] = r
        print(f"{name:8} new {r['new']['status']}", flush=True)
    assert results["short"]["old"]["seconds"] < TIMEOUT / 2, f"the old build waited {results['short']['old']}"
    assert results["check-dies"]["new"]["seconds"] < TIMEOUT / 2, f"a check outlived merl: {results['check-dies']['new']}"
    report_dir(out)
    for name in results:
        os.makedirs(os.path.join(out, name))
    report(results, out, "# selftest", names)
    text = open(os.path.join(out, "report.md")).read()
    for name, (_, _, _, verdict, holds) in SELFTEST.items():
        mine = [line for line in text.splitlines()
                if f"| {name} |" in line or line.startswith(f"- {name},") or f" {name} #" in line]
        assert f"| **{verdict}" in mine[0], f"{name}: want {verdict}, the row is {mine[0]}"
        assert holds in "\n".join(mine), f"{name}: the report never says {holds!r} of it"
        if verdict not in ("PASS", "DIFF") and name != "restarted":
            last = SGR.sub("", results[name]["new"].get("last", ["", ""])[1])
            assert "ready" in last, f"{name}: the last screen is not merl's: {last.strip()[-80:]!r}"
    assert results["end"]["new"]["last"][3] == list(SIZE), "the last checkpoint kept at another size"
    last = results["restarted"]["new"]["last"][0]
    assert last == "the screen after merl died", f"a merl dead on its restart shown by {last!r}"
    screens, one, fake = os.path.join(tmp, "screens"), {"pass": names["pass"]}, lambda n: os.path.join(tmp, n)
    for binary, d in (("other", "echo other;"), ("gone", "exit 3;")):
        open(fake(binary), "w").write(FAKE % {"q": "exit 0", "d": d, "s": ""})
        os.chmod(fake(binary), 0o755)
    with contextlib.redirect_stdout(io.StringIO()) as said:
        assert golden(fake("pass-new"), one, work, True, screens), "--update failed"
        for binary, want in (("pass-new", True), ("diff-red-new", True), ("other", False), ("gone", False)):
            assert golden(fake(binary), one, work, False, screens) == want, f"--golden on {binary}: not {want}"
    assert "+other" in said.getvalue() and "EXIT 3 at" in said.getvalue(), said.getvalue()
    open(fake("themed"), "w").write(FAKE % {"q": "exit 0", "s": "",
                                             "d": 'grep -qs github-light "$HOME/.config/merl/config.toml" && echo light;'})
    os.chmod(fake("themed"), 0o755)
    drawn = lambda run, theme: sorted(os.listdir(os.path.join(tmp, run, theme, "pass")))
    with contextlib.redirect_stdout(io.StringIO()):
        shot = shots(fake("themed"), fake("pass-new"), None, one, work, os.path.join(tmp, "shots"))
        shots(fake("themed"), fake("gone"), "gone", one, work, os.path.join(tmp, "shots-gone"))
    for theme, light in (("default", False), ("github-light", True)):
        assert ("light" in shot[theme]["pass"]["checkpoints"][-1][1]) == light, f"{theme}: the wrong theme"
        assert drawn("shots", theme) == (["02.png"] if light else []), f"{theme}: {drawn('shots', theme)}"
        assert drawn("shots-gone", theme) == ["02.png"], f"{theme} on gone: {drawn('shots-gone', theme)}"
    listed = open(os.path.join(tmp, "shots-gone", "shots.md")).read()
    assert "| default/pass/02.png |" in listed and "| new |" in listed, listed
    label = load(os.path.join(HERE, "go-d.steps"))[1][0][0]
    assert label.startswith("tests/smoke/go-d.steps:"), f"a step labelled {label!r}"
    hung = os.path.join(tmp, "hung-new")
    for sig, code, want in (
            (signal.SIGINT, f"run.play({hung!r}, {names['hung']!r}, {work!r})", None),
            (signal.SIGTERM, f"sys.argv = ['run.py', '--work', 'work']; "
                             f"run.played = lambda a, *_: list(run.pooled({{'hung': {names['hung']!r}}}, "
                             f"lambda n, s: run.play({hung!r}, {names['hung']!r}, a.work, slot=s))); run.main()",
             128 + signal.SIGTERM)):
        child = subprocess.Popen([sys.executable, "-B", "-c", f"import sys; sys.path.insert(0, {HERE!r}); "
                                  f"import run; {code}"], cwd=tmp,
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        tmux, end = ["tmux", "-L", f"smoke{child.pid}"], time.monotonic() + 10
        while "ready" not in subprocess.run([*tmux, "capture-pane", "-p", "-t", "0"],
                                            capture_output=True, text=True).stdout:
            assert time.monotonic() < end and child.poll() is None, f"{sig.name}: the fake never showed ready"
            time.sleep(POLL)
        socket = subprocess.run([*tmux, "display", "-p", "#{socket_path}"], capture_output=True, text=True)
        child.send_signal(sig)
        try:
            status = child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            child.kill()
            raise AssertionError(f"{sig.name}: the run went on")
        assert want is None or status == want, f"{sig.name}: exit {status}, want {want}"
        assert not servers_left(child.pid), f"{sig.name} left a tmux server in slots {servers_left(child.pid)}"
        assert not os.path.exists(socket.stdout.strip()), f"{sig.name} left the tmux socket file"
    assert open(os.path.join(work, "lock")).read() == "", "SIGTERM left the lock held"
    held, said = lock(work), lambda name: open(os.path.join(tmp, f"lock-{name}.txt")).read()
    waits, given, reports = (subprocess.Popen(
        [sys.executable, "-B", "-c", f"import sys; sys.path.insert(0, {HERE!r}); import run; run.WORK = {work!r}; "
         f"sys.argv = ['run.py', *{argv!r}]; run.played = lambda *_: print('played'); run.main()"],
        cwd=tmp, stdout=open(os.path.join(tmp, f"lock-{name}.txt"), "w"), stderr=subprocess.STDOUT)
        for name, argv in (("default", ["--golden"]), ("given", ["--golden", "--work", "work"]), ("report", [])))
    for name, run in (("given", given), ("report", reports)):
        assert run.wait(timeout=10) == 1 and "pass another --work" in said(name), said(name)
    end = time.monotonic() + 10
    while "waiting" not in said("default"):
        assert time.monotonic() < end and waits.poll() is None, f"no wait: {said('default')!r}"
        time.sleep(POLL)
    time.sleep(0.3)
    assert waits.poll() is None and "played" not in said("default"), "the default --work played under the lock"
    unlock(held)
    assert waits.wait(timeout=10) == 0 and "played" in said("default"), said("default")
    assert os.path.exists(os.path.join(out, "signal", "new-last.png")), "no last screen of the crash"
    assert not servers_left(os.getpid()), f"a tmux server outlived its play in slots {servers_left(os.getpid())}"
    shutil.rmtree(tmp)
    print("selftest ok")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--new", help="the build to check (default: this checkout, built)")
    p.add_argument("--old", help="the build to compare with, a binary or a tag (default: the last release)")
    p.add_argument("--work", help=f"the fixture and the copies merl runs in (default: {WORK})")
    p.add_argument("--out", help="the report and its PNGs (default: WORK/out, WORK/out-only for --only)")
    p.add_argument("--only", help="comma-separated scenario names, or parts of names")
    p.add_argument("--gif", help="record the one scenario --only names on the new build as this GIF")
    p.add_argument("--golden", action="store_true",
                   help="the new build alone, every checkpoint against tests/smoke/screens/ (CI)")
    p.add_argument("--update", action="store_true", help="--golden, writing the screens instead of comparing")
    p.add_argument("--all-shots", action="store_true",
                   help="each checkpoint that differs from the old build as a PNG, in the default and a light theme")
    p.add_argument("--again", action="store_true", help="the report again from OUT/results.json, unplayed")
    p.add_argument("--selftest", action="store_true", help="the runner's failure paths on fake merls")
    a = p.parse_args()
    a.golden |= a.update
    if a.selftest:
        return selftest()
    wait, a.work = a.work is None and (a.golden or a.gif), os.path.abspath(a.work or WORK)
    out = os.path.abspath(a.out or os.path.join(a.work, ("shots" if a.all_shots else "out") + "-only" * bool(a.only)))
    names = {f[:-6]: os.path.join(HERE, f) for f in sorted(os.listdir(HERE)) if f.endswith(".steps")}
    if a.again:
        results, head = json.load(open(os.path.join(out, "results.json")))
        return report(results, out, head, names)
    t0, load0 = time.monotonic(), os.getloadavg()[0]
    only = a.only.split(",") if a.only else []
    picked = {n: f for n, f in names.items()
              if (not only or any(o == n or o not in names and o in n for o in only))
              and not (a.golden and n in RELEASE_ONLY)}
    if not picked:
        sys.exit(f"no scenario is named or holds {a.only}")
    held = lock(a.work, wait)
    for sig in (signal.SIGTERM, signal.SIGHUP):
        if signal.getsignal(sig) != signal.SIG_IGN:
            signal.signal(sig, lambda n, _: sys.exit(128 + n))
    try:
        played(a, picked, names, out, t0, load0)
    finally:
        unlock(held)


def played(a, picked, names, out, t0, load0):
    if not a.new:
        subprocess.run(["cargo", "build", "--release", "--locked"], cwd=ROOT, check=True)
    new = a.new or os.path.join(ROOT, "target", "release", "merl")
    subprocess.run([os.path.join(HERE, "setup.sh"), os.path.join(a.work, "base")], check=True)
    if any(load(f)[0] in DEMOS for f in picked.values()):
        subprocess.run([os.path.join(ROOT, "assets", "tapes", "setup.sh"), os.path.join(a.work, "demo")],
                       check=True, stdout=subprocess.DEVNULL)
    if not version(new):
        sys.exit(f"{new} does not answer --version")
    if a.gif:
        assert len(picked) == 1, f"--gif records one scenario, --only picked {list(picked) or 'none'}"
        return gif(new, *picked.values(), a.work, a.gif)
    if a.golden:
        orphans = sorted(set(os.listdir(SCREENS)) - set(names) | set(os.listdir(SCREENS)) & set(RELEASE_ONLY))
        if orphans:
            sys.exit(f"tests/smoke/screens holds {', '.join(orphans)}, no scenario --golden plays: delete it")
        return golden(new, picked, a.work, a.update) or sys.exit(1)
    old, tag = (a.old, None) if a.old and os.path.exists(a.old) else last_release(a.old)
    if not version(old):
        sys.exit(f"{old} does not answer --version")
    if a.all_shots:
        return shots(new, old, tag, picked, a.work, out)
    report_dir(out)
    results = {}
    describe = ["git", "describe", "--tags", "--always", "--dirty"]
    built = "" if a.new else ", " + subprocess.run(describe, cwd=ROOT, capture_output=True, text=True).stdout
    for name, r in pooled(picked, lambda name, slot: scenario(picked[name], new, old, a.work, slot)):
        os.makedirs(os.path.join(out, name))
        results[name] = r
        results = {n: results[n] for n in picked if n in results}
        print(f"{name:12} new {r['new']['status']} | old {r['old']['status']}", flush=True)
        wall = time.monotonic() - t0
        head = (f"# merl smoke: new vs old\n\nnew `{os.path.relpath(new, ROOT)}` ({version(new)}{built.strip()})"
                f" · old {tag or '`' + old + '`'} ({version(old)}) · {len(results)} of {len(picked)} scenarios"
                f" · wall {wall:.0f} s, the build included · load {load0:.1f} → {os.getloadavg()[0]:.1f}")
        json.dump([results, head], open(os.path.join(out, "results.json"), "w"))
    report(results, out, head, names)
    print(os.path.join(out, "report.md"))


if __name__ == "__main__":
    main()
