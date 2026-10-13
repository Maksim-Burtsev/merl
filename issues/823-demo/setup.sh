#!/bin/bash
# Issue #823 demo: a branch reviewed in round 1 (six of its eight files viewed), then the author pushes.
set -e
P=/tmp/823/parcel
rm -rf /tmp/823 && mkdir -p $P && cd $P
git init -q -b main
git config user.name Dev; git config user.email dev@example.com
mkdir -p app tests docs
w() { cat > "$1"; }
w app/__init__.py <<'X'
X
w app/config.py <<'X'
import os


class Settings:
    def __init__(self):
        self.db_url = os.environ.get("DB_URL", "sqlite:///parcel.db")
        self.timeout = float(os.environ.get("TIMEOUT", "5"))
        self.workers = int(os.environ.get("WORKERS", "4"))


settings = Settings()
X
w app/queue.py <<'X'
from collections import deque


class Queue:
    def __init__(self):
        self.items = deque()

    def push(self, job):
        self.items.append(job)

    def pop(self):
        return self.items.popleft() if self.items else None

    def __len__(self):
        return len(self.items)
X
w app/client.py <<'X'
import json
import urllib.request

from app.config import settings


def post(url, payload):
    data = json.dumps(payload).encode()
    req = urllib.request.Request(url, data=data, method="POST")
    req.add_header("Content-Type", "application/json")
    with urllib.request.urlopen(req, timeout=settings.timeout) as resp:
        return json.loads(resp.read())
X
w app/worker.py <<'X'
from app.client import post
from app.queue import Queue

queue = Queue()


def run_once():
    job = queue.pop()
    if job is None:
        return False
    post(job["url"], job["payload"])
    return True
X
w app/models.py <<'X'
from dataclasses import dataclass


@dataclass
class Parcel:
    id: str
    weight: float
    address: str
X
w tests/test_queue.py <<'X'
from app.queue import Queue


def test_push_pop():
    q = Queue()
    q.push(1)
    assert q.pop() == 1
    assert q.pop() is None
X
w docs/README.md <<'X'
# parcel

Sends parcel events to the courier API.
X
git add -A && git commit -qm "parcel service"
git checkout -qb fix/retries
python3 - <<'PY'
import re
def ed(p, a, b):
    s = open(p).read(); assert a in s, (p, a); open(p, "w").write(s.replace(a, b, 1))
ed("app/config.py", '        self.workers = int(os.environ.get("WORKERS", "4"))\n',
   '        self.workers = int(os.environ.get("WORKERS", "4"))\n        self.retries = int(os.environ.get("RETRIES", "3"))\n        self.backoff = float(os.environ.get("BACKOFF", "0.5"))\n')
ed("app/queue.py", "    def pop(self):", "    def push_front(self, job):\n        self.items.appendleft(job)\n\n    def pop(self):")
ed("app/client.py", "import json\n", "import json\nimport time\n")
ed("app/client.py", "def post(url, payload):\n", "class Retry(Exception):\n    pass\n\n\ndef post(url, payload):\n")
ed("app/worker.py", "    post(job[\"url\"], job[\"payload\"])\n    return True",
   "    try:\n        post(job[\"url\"], job[\"payload\"])\n    except OSError:\n        job[\"tries\"] = job.get(\"tries\", 0) + 1\n        queue.push_front(job)\n        return False\n    return True")
ed("app/models.py", "    address: str\n", "    address: str\n    tries: int = 0\n")
ed("docs/README.md", "API.\n", "API.\n\nA failed post is retried `RETRIES` times, `BACKOFF` seconds apart.\n")
open("tests/test_worker.py", "w").write('''from app import worker


def test_failed_post_goes_back(monkeypatch):
    def fail(url, payload):
        raise OSError("down")

    monkeypatch.setattr(worker, "post", fail)
    worker.queue.push({"url": "u", "payload": {}})
    assert worker.run_once() is False
    assert worker.queue.pop()["tries"] == 1
''')
ed("tests/test_queue.py", "    assert q.pop() is None\n", "    assert q.pop() is None\n\n\ndef test_push_front():\n    q = Queue()\n    q.push(1)\n    q.push_front(2)\n    assert q.pop() == 2\n")
PY
git add -A && git commit -qm "retry failed posts"
