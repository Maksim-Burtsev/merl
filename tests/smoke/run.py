#!/usr/bin/env python3
"""The smoke test before a release: plays every scenario in tmux on the new build and on the last
release, and writes what differs to a report (.claude/skills/smoke-test/SKILL.md reads it).

    tests/smoke/run.py                           build this checkout, fetch the last release, play all
    tests/smoke/run.py --only review,edit        these two, or all whose name holds a part given
    tests/smoke/run.py --old target/release/merl the new build against itself
    tests/smoke/run.py --gif demo.gif --only python-d    one scenario on the new build, as a GIF
    tests/smoke/run.py --selftest                the runner's own failure paths, on fake binaries

Without --new the checkout is built (`cargo build --release --locked`, its own target/); without
--old the last GitHub release's macOS binary is downloaded once into ~/.cache/merl-smoke/TAG/,
checked against the release's sha256 and against `--version` before it is kept. A run rebuilds
the fixture (setup.sh) in WORK, /tmp/merl-smoke by default: a short path that is the same on
every run, so the two builds' screens can be compared. A lock on WORK refuses a second run while
one plays. The report goes to WORK/out, or WORK/out-only for --only, so a rerun of one scenario
leaves the full report in place; a directory --out names is deleted only if a run made it.

Each scenario is played on the new build, then the old one, from a fresh copy of the fixture in a
tmux pane of 185x55, a laptop's full screen, with an empty HOME. A scenario is a steps file, one
step per line, `#` comments and blank lines skipped; the grammar of tools/cast.py and
assets/tapes/record.py plus a few verbs of its own:

    merl ARGS          start merl in the project; later in the file: quit it (it must exit 0)
                       and start it again, the same HOME kept
    wait TEXT          an assertion and a checkpoint: the pane shows TEXT within 10 s, and the
                       screen is saved once it has settled; a timeout is a FAIL, merl dead a CRASH
    key K*N            tmux key names, each N times (`key@P` paces a GIF, here keys go 10 ms apart)
    type TEXT          the characters: at once here, one keystroke each in a GIF
    sleep S            seconds, at most 0.3: the waits do the syncing
    run CMD            a shell command in the project, before the next step: the agent in the next
                       split, or a check of the bytes on disk; a non-zero exit fails the scenario.
                       $SMOKE_TMUX is the tmux command of merl's pane: `$SMOKE_TMUX show-buffer`
                       holds what OSC 52 copied
    size WxH           the pane's size: before the first `merl`, the size it starts at
    time               the next wait is a timed step: three samples on each build, the fastest kept
    caption TEXT       skipped here; --gif draws it under the screen from then on, over the step
    show               skipped (record.py's)
    include FILE       the steps of FILE, relative to the repository
    project gitea      header: run in the README's gitea checkout, made by assets/tapes/setup.sh
                       (fetched once, 59 MB), instead of the fixture

A scenario that is not about the tree hides it (`t`) after its first wait, so that a change to the
tree shows in one checkpoint per scenario, not in all of them. What a wait may name, and where a new
feature's steps go: AGENTS.md, its bullets on tests/smoke.
"""
import argparse, difflib, fcntl, hashlib, json, os, platform, re, shlex, shutil, signal, subprocess, sys
import tempfile, threading, time, traceback

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
HERE = os.path.join(ROOT, "tests", "smoke")
sys.path.insert(0, os.path.join(ROOT, "tools"))
sys.dont_write_bytecode = True  # no __pycache__ left in tools/
import cast  # noqa: E402
from PIL import Image, ImageDraw, ImageFont  # noqa: E402

CACHE = os.path.expanduser("~/.cache/merl-smoke")
PACE, POLL, TIMEOUT, SLEEP_CAP, SIZE = 0.01, 0.02, 10.0, 0.3, (185, 55)
HOLD = 1.2  # --gif: seconds each checkpoint stays on screen
MARK = ".merl-smoke-report"  # in every report directory a run made, and only there
SGR = re.compile(r"\x1b\[[0-9;]*m")
# What differs from run to run whatever the build: the tutor's sample project is unpacked into a
# directory named after merl's pid. Masked to the same width, so the status line keeps its shape.
VOLATILE = re.compile(r"(?<=merl-tutor-)\d+")
# The `run` steps commit and push as the agent, at a fixed date: the same hashes on every run.
AGENT = {"GIT_CONFIG_GLOBAL": "/dev/null", "GIT_AUTHOR_NAME": "agent", "GIT_COMMITTER_NAME": "agent",
         "GIT_AUTHOR_EMAIL": "agent@example.com", "GIT_COMMITTER_EMAIL": "agent@example.com",
         "GIT_AUTHOR_DATE": "2026-09-03T10:00:00Z", "GIT_COMMITTER_DATE": "2026-09-03T10:00:00Z"}
# Every verdict the table can show, printed under it.
LEGEND = [("PASS", "every checkpoint the same on both builds"),
          ("DIFF", "the new build played it through; the differences below each need a verdict"),
          ("FAIL", "a wait timed out: the text never showed"),
          ("CRASH", "merl died: a panic (101) or a signal, named"),
          ("EXIT", "merl exited with another status, on its own or on q"),
          ("HUNG", "merl did not quit on q"),
          ("RUN", "a `run` step failed: wrong bytes on disk, or the agent's command"),
          ("ERROR", "the runner failed, not merl: rerun, fix run.py if it comes back")]


class Merl(cast.Pane):
    """cast.Pane with a pane that stays when merl dies (its last screen and exit status kept),
    stderr appended to a file, and merl restartable in place."""

    def __init__(self, binary, args, home, project, size, err):
        self.sock, self.home, self.binary, self.err = "smoke%d" % os.getpid(), home, binary, err
        self.size = size
        options = [("remain-on-exit", "on"), ("status", "off"), ("default-shell", "/bin/sh"),
                   ("remain-on-exit-format", "merl exited: #{pane_dead_status}")]
        self.tmux("start-server", *[w for o in options for w in (";", "set", "-g", *o)],
                  ";", "set", "-s", "set-clipboard", "on",  # OSC 52 into a tmux buffer
                  ";", "new-session", "-d", "-x", str(size[0]), "-y", str(size[1]), "-c", project,
                  self.command(args))
        self.socket = self.tmux("display", "-p", "#{socket_path}").strip()

    def tmux(self, *args):
        """cast.Pane's, raising instead of exiting: a tmux failure is the scenario's ERROR."""
        out = subprocess.run(["tmux", "-L", self.sock, "-f", "/dev/null", *args],
                             capture_output=True, text=True)
        if out.returncode:
            raise RuntimeError("tmux %s: %s" % (args[0], out.stderr.strip()))
        return out.stdout

    def command(self, args):
        # XDG_* would point merl and git at the user's own config; GOTOOLCHAIN=local keeps `go env`
        # off the network whatever go.mod asks for.
        unset = " ".join("-u " + v for v in os.environ if v.startswith("XDG_"))
        return "exec env %s HOME=%s COLORTERM=truecolor GOTOOLCHAIN=local %s 2>>%s" % (
            unset, shlex.quote(self.home), shlex.join([os.path.abspath(self.binary)] + args),
            shlex.quote(self.err))

    def restart(self, args):
        self.tmux("respawn-pane", "-k", "-t", "0", self.command(args))

    def dead(self):
        """None while merl runs, else its exit status, 128 + N for signal N."""
        f = "#{pane_dead},#{pane_dead_status},#{pane_dead_signal}"
        dead, status, sig = self.tmux("display", "-p", "-t", "0", f).strip().split(",")
        if dead != "1":
            return None
        if sig:  # tmux 3.7 names it, `abrt`; older ones print the number
            return 128 + (int(sig) if sig.isdigit() else getattr(signal, "SIG" + sig.upper(), 0))
        return int(status or 0)

    def frame(self):
        capture, cursor = super().frame()
        return VOLATILE.sub(lambda m: "N" * len(m.group()), capture), cursor

    def settled(self):
        """The screen once two captures 50 ms apart agree (1 s at most)."""
        last, end = self.frame(), time.monotonic() + 1
        while time.monotonic() < end:
            time.sleep(0.05)
            if (now := self.frame()) == last:
                break
            last = now
        return last

    def quit(self):
        """q, and a second q for unsaved edits; merl's exit status, None if it would not go."""
        for keys in (["Escape", "Escape", "q"], ["q"]):
            if self.dead() is None:
                self.tmux("send-keys", "-t", "0", *keys)
                end = time.monotonic() + 1.5
                while self.dead() is None and time.monotonic() < end:
                    time.sleep(POLL)
        return self.dead()

    def close(self):  # cast.Pane's would delete HOME, which the next play copies afresh anyway
        subprocess.run(["tmux", "-L", self.sock, "kill-server"], capture_output=True)
        if os.path.exists(self.socket):  # tmux 3.7 leaves the socket file behind
            os.unlink(self.socket)


def died(status):
    """The verdict of a merl that is gone, or would not go."""
    if status is None:
        return "HUNG"
    if status >= 128:
        try:
            return f"CRASH {signal.Signals(status - 128).name}"
        except ValueError:
            return f"CRASH signal {status - 128}"
    return f"CRASH {status}" if status == 101 else f"EXIT {status}"


def load(path):
    """(project, steps) of a scenario: steps as (line, verb, pace, arg), includes expanded."""
    project, steps = None, []
    # from the repository's root: assets/review.steps and tests/smoke/review.steps are two files
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


def fresh(work, project):
    """WORK/run as the fixture has it (the project, its origin, an empty HOME), whatever the last
    run left there; and the directory merl runs in."""
    run = os.path.join(work, "run")
    shutil.rmtree(run, ignore_errors=True)
    shutil.copytree(os.path.join(work, "base"), run, symlinks=True)
    if project == "gitea":
        gitea = os.path.join(work, "demo", "gitea")
        subprocess.run("git checkout -q -- . && git clean -fdq", shell=True, cwd=gitea, check=True)
        return gitea
    return os.path.join(run, "orders")


class Recorder:
    """--gif: samples the pane ten times a second, each frame with the band of its moment: the
    scenario's caption, and the step the runner is on."""

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


def play(binary, path, work, timed_only=False, rec=None):
    """One run of a scenario on one binary. Whatever happens, an interrupt included, the pane's tmux
    server is gone after it. A play that does not end `ok` keeps in `last` the screen as it was at
    the failure: before any q, and for a merl that died, its last checkpoint since it started."""
    home, err = os.path.join(work, "run", "home"), os.path.join(work, "run", "stderr")
    r = {"checkpoints": [], "times": {}, "slow": [], "status": "ok", "timed": False}
    size, pane, timed, t0 = list(SIZE), None, False, time.monotonic()
    # seen: the last checkpoint of the merl now running, its size included. A dying merl restores
    # the terminal first, so a frame taken at its death, or a poll just before, can already be the
    # blank screen behind it.
    # gone: a wait saw merl exit (`wait merl exited: 0`), so its death is the scenario's, not a failure.
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
        cwd = fresh(work, project)
        open(err, "w").close()
        last_timed = max((i for i, s in enumerate(steps) if s[1] == "time"), default=-1)
        r["timed"] = last_timed >= 0
        for i, (where, verb, pace, arg) in enumerate(steps):
            step = f"{where} `{verb} {arg}`"
            if rec and not rec.thread and pane and verb not in ("wait", "sleep", "caption"):
                rec.start(pane)
            # a merl that died on a key no wait followed: reported as a wait would, not on the next step
            if pane and not gone and verb in ("merl", "key", "type", "run") and (s := pane.dead()) is not None:
                r["status"] = f"{died(s)} before {step}"
                keep_dead()
                break
            if verb == "merl":
                if pane is None:
                    pane = Merl(binary, shlex.split(arg), home, cwd, size, err)
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
                ms, dead_at = None, None
                while time.monotonic() - last_key < TIMEOUT:
                    if arg in pane.tmux("capture-pane", "-p", "-t", "0"):
                        ms = round((time.monotonic() - last_key) * 1000)
                        break
                    if pane.dead() is not None:  # a moment for tmux to draw `merl exited`
                        dead_at = dead_at or time.monotonic()
                        if time.monotonic() - dead_at > 0.5:
                            break
                    time.sleep(POLL)
                if ms is None:
                    s = pane.dead()
                    r["status"] = f"{'FAIL' if s is None else died(s)} at {step}"
                    r["checkpoints"].append([f"{where} wait {arg} (never shown)", *pane.frame(), size])
                    if s is None:
                        keep(f"{where} wait {arg}: the screen when it gave up", pane.frame())
                    else:
                        keep_dead()
                    break
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
                keys = [["-l", c] for c in arg] if verb == "type" else [
                    [k.partition("*")[0]] for k in arg.split()
                    for _ in range(int(k.partition("*")[2] or 1))]
                if verb == "type" and not rec:  # a burst, as a paste arrives: a keystroke a char is slow
                    keys = [["-l", arg]]
                gap = float(pace or (0.12 if verb == "type" else 0.25)) if rec else PACE
                if rec:
                    rec.step = f"{verb} {arg}"
                for k in keys:
                    pane.tmux("send-keys", "-t", "0", *k)
                    time.sleep(gap)
                last_key = time.monotonic()
            elif verb == "sleep":
                time.sleep(float(arg) if rec else min(float(arg), SLEEP_CAP))
            elif verb == "run":
                if rec:
                    rec.step = f"run {arg[:90]}"
                env = {**os.environ, **AGENT, "HOME": home,
                       "SMOKE_TMUX": f"tmux -L {pane.sock} -f /dev/null"}
                out = subprocess.run(arg, shell=True, cwd=cwd, env=env, capture_output=True, text=True)
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
                pane.close()  # a KeyboardInterrupt in quit() still gets here
    r["seconds"] = round(time.monotonic() - t0, 1)
    r["stderr"] = ((open(err).read() if os.path.exists(err) else "") + runner)[-2000:]
    return r


def scenario(path, new, old, work):
    """A scenario on both builds, and twice more on each for its timed steps: old/new, then
    new/old, each up to its last timed wait. A rerun that does not end `ok` is the scenario's
    status, with its stderr and last screen, unless the first play failed already."""
    res = {s: play(b, path, work) for s, b in (("new", new), ("old", old))}
    if res["new"]["timed"]:
        for side in res:
            res[side]["samples"] = {k: [v] for k, v in res[side]["times"].items()}
        for n, order in enumerate((("old", "new"), ("new", "old")), 2):
            for side in order:
                again = play(new if side == "new" else old, path, work, timed_only=True)
                for k, v in again["times"].items():
                    res[side]["samples"].setdefault(k, []).append(v)
                res[side]["stderr"] += again["stderr"]
                if again["status"] != "ok" and res[side]["status"] == "ok":
                    res[side]["status"] = f"{again['status']} (play {n} of 3, for the timed steps)"
                    res[side]["last"] = again.get("last")
    return res


def hexed(v):
    return "#%02x%02x%02x" % v if isinstance(v, tuple) else str(v)


def boxes(old, new):
    """What differs, as boxes of cells joined across gaps of 3 columns and 1 row, each cut out of
    both screens and diffed: a tree that gained a row is one `+` line, not twenty changed rows.
    Each is (key, what, where): a difference is known by its key, the text of a text change and
    the colours of a colour change (old → new), so two tints over the same text stay apart."""
    size = new[3]
    g = [cast.parse(c[1], *size) for c in (old, new)]
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
        for _ in range(24):  # out to the word's ends, so `3:45` does not read `5`; not past a border
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
    """The screens of `pair`, (name, checkpoint) each, one above the other, a red mark beside each
    row in `rows`."""
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
    """The `## [Unreleased]` entries of CHANGELOG.md: (first words, issues, scenarios citing one)."""
    text = open(os.path.join(ROOT, "CHANGELOG.md")).read()
    part = text.split("## [Unreleased]", 1)[1].split("\n## [", 1)[0]
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
            failed = "(never shown)" in o[0] + n[0]
            colour = SGR.sub("", o[1]) == SGR.sub("", n[1]) and o[2:] == n[2:]
            kinds.append("failed" if failed else "colour" if colour else "text")
            what, diff_rows = boxes(o, n)
            png_pair([("old", o), ("new", n)], diff_rows, os.path.join(out, name, f"{i:02d}.png"), fonts, cell)
            # The same change at another spot of another screen is one entry.
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
    """[1, 2, 3, 5] as `1–3, 5`."""
    out, start = [], None
    for i, n in enumerate(nums):
        start = n if start is None else start
        if i + 1 == len(nums) or nums[i + 1] != n + 1:
            out.append(f"{start}–{n}" if n > start else str(n))
            start = None
    return ", ".join(out)


def version(binary):
    """What `binary --version` prints, or None when it does not run and say `merl`."""
    try:
        out = subprocess.run([binary, "--version"], capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return out.stdout.strip() if out.returncode == 0 and out.stdout.startswith("merl") else None


def last_release(tag=None):
    """The macOS binary of a release (the latest by default), from CACHE/TAG/merl. A copy that
    does not answer `--version` is fetched again; a fetched one is kept only once its archive
    matches the release's sha256 and what it unpacks to answers `--version`."""
    gh = lambda *a: subprocess.run(["gh", *a], cwd=ROOT, capture_output=True, text=True)  # noqa: E731
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
        sums = os.path.join(tmp, f"{asset}.sha256")  # a release before 0.7.0 may have none
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


def lock(work):
    """Holds WORK for this run; a second run is refused, told which run holds it."""
    os.makedirs(work, exist_ok=True)
    f = open(os.path.join(work, "lock"), "a+")
    try:
        fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        f.seek(0)
        sys.exit(f"{work} is in use by another smoke run: {f.read().strip() or 'unknown'}.\n"
                 "Wait for it to end, or pass --work DIR.")
    f.truncate(0)
    f.write(f"pid {os.getpid()}, {' '.join(sys.argv)}, from {os.getcwd()}, since {time.ctime()}\n")
    f.flush()
    return f  # the lock lasts as long as the file stays open: `unlock` it, or exit


def unlock(f):
    """Gives WORK back. The file stays, empty: deleting it would let a run that opened it before the
    deletion lock the old file while the next run locks a new one."""
    f.truncate(0)
    f.close()


def report_dir(out):
    """OUT emptied for a new report: made here, or a report directory a run made before."""
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
    merged = []  # identical frames in a row are one, held for as long as they lasted
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


# --selftest: fake merls, bash scripts that print `ready` and act on the keys they read.
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
  esac
done
"""
# name: (steps, new merl's start, `d` and `q`, the old one's, the verdict, a text the report must hold)
SELFTEST = {
    "pass": ("merl\nwait ready\nkey d\nwait done\n", {}, {}, "PASS", ""),
    "signal": ("merl\nwait ready\nkey a\nwait done\n", {}, {}, "CRASH", "CRASH SIGABRT at"),
    "exit": ("merl\nwait ready\nkey e\nwait done\n", {}, {}, "EXIT", "EXIT 3 at"),
    "fail": ("merl\nwait ready\nwait never\n", {}, {}, "FAIL", "FAIL at"),
    "hung": ("merl\nwait ready\n", {"q": ":"}, {}, "HUNG", "HUNG on q"),
    "run": ("merl\nwait ready\nrun test -e nowhere\n", {}, {}, "RUN", "RUN 1 at"),
    # dead on a key no wait follows: the death, not the q after it, with the screen at its size
    "end": ("merl\nwait ready\nsize 60x10\nkey e\n", {}, {}, "EXIT", "EXIT 3 after its last step"),
    "restart": ("merl\nwait ready\nkey e\nmerl\nwait ready\n", {}, {}, "EXIT", "EXIT 3 before"),
    # quit on purpose and waited for: a pass, with a step and a restart after it; another status fails
    "quit": ("merl\nwait ready\nkey q\nwait merl exited: 0\nrun true\nmerl\nwait ready\n", {}, {}, "PASS", ""),
    "quit-3": ("merl\nwait ready\nkey e\nwait merl exited: 0\n", {}, {}, "EXIT", "EXIT 3 at"),
    # dead on its second start: not shown with the first merl's screen
    "restarted": ("merl\nwait ready\nmerl\nwait ready\n",
                  {"s": '[ -e "$0.started" ] && exit 101; touch "$0.started"'}, {}, "CRASH", "CRASH 101 at"),
    # the first play passes, the two reruns for the timed step exit 101 at `d`
    "rerun": ("merl\nwait ready\nkey d\ntime\nwait done\n",
              {"d": '[ -e "$0.played" ] && exit 101; touch "$0.played";'}, {}, "CRASH",
              "(play 2 of 3, for the timed steps)"),
    "slower": ("merl\nwait ready\nkey d\ntime\nwait done\n", {"d": "sleep 0.5;"}, {}, "PASS", "**slower**"),
    # the builds draw `done` in different colours: two tints over one text are two differences
    "diff-red": ("merl\nwait ready\nkey d\nwait done\n", {"d": "printf '\\033[31m';"}, {}, "DIFF", "#ff757f"),
    "diff-blue": ("merl\nwait ready\nkey d\nwait done\n", {"d": "printf '\\033[34m';"}, {}, "DIFF", "#82aaff"),
    # tmux gone under a play: the runner's ERROR, and the run goes on
    "error": ("merl\nwait ready\nrun $SMOKE_TMUX kill-server\nwait ready\n", {}, {}, "ERROR", "tmux capture-pane"),
}


def selftest():
    """Plays SELFTEST's scenarios on fake merls and checks each verdict: seconds, no fixture."""
    global TIMEOUT
    TIMEOUT = 2.0
    tmp = tempfile.mkdtemp(prefix="smoke-selftest-")
    work, out = os.path.join(tmp, "work"), os.path.join(tmp, "out")
    for d in ("orders", "home"):
        os.makedirs(os.path.join(work, "base", d))
    names, results = {}, {}
    for name, (steps, new, old, _, _) in SELFTEST.items():
        names[name] = os.path.join(tmp, name + ".steps")
        open(names[name], "w").write(steps)
        bins = []
        for side, acts in (("new", new), ("old", old)):
            bins.append(os.path.join(tmp, f"{name}-{side}"))
            open(bins[-1], "w").write(FAKE % {"q": "exit 0", "d": "", "s": "", **acts})
            os.chmod(bins[-1], 0o755)
        results[name] = scenario(names[name], *bins, work)
        print(f"{name:8} new {results[name]['new']['status']}", flush=True)
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
        if verdict not in ("PASS", "DIFF") and name != "restarted":  # the screen at the failure,
            last = SGR.sub("", results[name]["new"].get("last", ["", ""])[1])  # not the blank one after q
            assert "ready" in last, f"{name}: the last screen is not merl's: {last.strip()[-80:]!r}"
    assert results["end"]["new"]["last"][3] == list(SIZE), "the last checkpoint kept at another size"
    last = results["restarted"]["new"]["last"][0]
    assert last == "the screen after merl died", f"a merl dead on its restart shown by {last!r}"
    label = load(os.path.join(HERE, "go-d.steps"))[1][0][0]
    assert label.startswith("tests/smoke/go-d.steps:"), f"a step labelled {label!r}"
    # Ctrl-C while a play waits for a merl that will not quit: its tmux server goes all the same.
    # SIGTERM through main, with a relative --work: the same, and the lock given back.
    hung = os.path.join(tmp, "hung-new")
    for sig, code, want in (
            (signal.SIGINT, f"run.play({hung!r}, {names['hung']!r}, {work!r})", None),
            (signal.SIGTERM, f"sys.argv = ['run.py', '--work', 'work']; "
                             f"run.played = lambda a, *_: run.play({hung!r}, {names['hung']!r}, a.work); run.main()",
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
        assert subprocess.run([*tmux, "ls"], capture_output=True).returncode, f"{sig.name} left a tmux server"
        assert not os.path.exists(socket.stdout.strip()), f"{sig.name} left the tmux socket file"
    assert open(os.path.join(work, "lock")).read() == "", "SIGTERM left the lock held"
    assert os.path.exists(os.path.join(out, "signal", "new-last.png")), "no last screen of the crash"
    left = subprocess.run(["tmux", "-L", f"smoke{os.getpid()}", "ls"], capture_output=True)
    assert left.returncode, "a tmux server outlived its play"
    shutil.rmtree(tmp)
    print("selftest ok")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--new", help="the build to check (default: this checkout, built)")
    p.add_argument("--old", help="the build to compare with, a binary or a tag (default: the last release)")
    p.add_argument("--work", default="/tmp/merl-smoke", help="the fixture and the copies merl runs in")
    p.add_argument("--out", help="the report and its PNGs (default: WORK/out, WORK/out-only for --only)")
    p.add_argument("--only", help="comma-separated scenario names, or parts of names")
    p.add_argument("--gif", help="record the one scenario --only names on the new build as this GIF")
    p.add_argument("--again", action="store_true", help="the report again from OUT/results.json, unplayed")
    p.add_argument("--selftest", action="store_true", help="the runner's failure paths on fake merls")
    a = p.parse_args()
    if a.selftest:
        return selftest()
    # absolute once: HOME and stderr go into a command that runs in the project, not here
    a.work = os.path.abspath(a.work)
    out = os.path.abspath(a.out or os.path.join(a.work, "out-only" if a.only else "out"))
    names = {f[:-6]: os.path.join(HERE, f) for f in sorted(os.listdir(HERE)) if f.endswith(".steps")}
    if a.again:
        results, head = json.load(open(os.path.join(out, "results.json")))
        return report(results, out, head, names)
    t0, load0 = time.monotonic(), os.getloadavg()[0]
    only = a.only.split(",") if a.only else []  # a scenario's own name, else a part of names
    picked = {n: f for n, f in names.items()
              if not only or any(o == n or o not in names and o in n for o in only)}
    if not picked:
        sys.exit(f"no scenario is named or holds {a.only}")
    held = lock(a.work)
    # SIGTERM and SIGHUP end the run the way Ctrl-C does: through every `finally`, so each play's
    # tmux server and its merl go, and the lock is given back.
    # A signal the run inherited as ignored (`nohup`) stays ignored.
    for sig in (signal.SIGTERM, signal.SIGHUP):
        if signal.getsignal(sig) != signal.SIG_IGN:
            signal.signal(sig, lambda n, _: sys.exit(128 + n))
    try:
        played(a, picked, names, out, t0, load0)
    finally:
        unlock(held)


def played(a, picked, names, out, t0, load0):
    """The run proper, once WORK is held: build, fixture, both builds' plays, the report."""
    if not a.new:
        subprocess.run(["cargo", "build", "--release", "--locked"], cwd=ROOT, check=True)
    new = a.new or os.path.join(ROOT, "target", "release", "merl")
    subprocess.run([os.path.join(HERE, "setup.sh"), os.path.join(a.work, "base")], check=True)
    if any(load(f)[0] == "gitea" for f in picked.values()):
        subprocess.run([os.path.join(ROOT, "assets", "tapes", "setup.sh"), os.path.join(a.work, "demo")],
                       check=True, stdout=subprocess.DEVNULL)
    if not version(new):
        sys.exit(f"{new} does not answer --version")
    if a.gif:
        assert len(picked) == 1, f"--gif records one scenario, --only picked {list(picked) or 'none'}"
        return gif(new, *picked.values(), a.work, a.gif)
    old, tag = (a.old, None) if a.old and os.path.exists(a.old) else last_release(a.old)
    if not version(old):
        sys.exit(f"{old} does not answer --version")
    report_dir(out)
    results = {}
    describe = ["git", "describe", "--tags", "--always", "--dirty"]
    built = "" if a.new else ", " + subprocess.run(describe, cwd=ROOT, capture_output=True, text=True).stdout
    for name, path in picked.items():
        os.makedirs(os.path.join(out, name))
        results[name] = scenario(path, new, old, a.work)
        print(f"{name:12} new {results[name]['new']['status']} | old {results[name]['old']['status']}",
              flush=True)
        wall = time.monotonic() - t0
        head = (f"# merl smoke: new vs old\n\nnew `{os.path.relpath(new, ROOT)}` ({version(new)}{built.strip()})"
                f" · old {tag or '`' + old + '`'} ({version(old)}) · {len(results)} of {len(picked)} scenarios"
                f" · wall {wall:.0f} s, the build included · load {load0:.1f} → {os.getloadavg()[0]:.1f}")
        # after every scenario, so a run cut short leaves what it played for --again
        json.dump([results, head], open(os.path.join(out, "results.json"), "w"))
    report(results, out, head, names)
    print(os.path.join(out, "report.md"))


if __name__ == "__main__":
    main()
