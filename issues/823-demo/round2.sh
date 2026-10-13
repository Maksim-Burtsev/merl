#!/bin/bash
# The author pushes two commits of fixes: two files you viewed, one you did not.
set -e
cd /tmp/823/parcel
python3 - <<'PY'
def ed(p, a, b):
    s = open(p).read(); assert a in s, (p, a); open(p, "w").write(s.replace(a, b, 1))
ed("app/worker.py", "    except OSError:\n        job[\"tries\"] = job.get(\"tries\", 0) + 1\n        queue.push_front(job)\n",
   "    except OSError:\n        job[\"tries\"] = job.get(\"tries\", 0) + 1\n        if job[\"tries\"] < settings.retries:\n            queue.push_front(job)\n")
ed("app/worker.py", "from app.client import post\n", "from app.client import post\nfrom app.config import settings\n")
ed("app/config.py", '"BACKOFF", "0.5"', '"BACKOFF", "1.0"')
PY
git commit -qam "stop after RETRIES tries"
python3 - <<'PY'
def ed(p, a, b):
    s = open(p).read(); assert a in s, (p, a); open(p, "w").write(s.replace(a, b, 1))
ed("tests/test_worker.py", "    assert worker.queue.pop()[\"tries\"] == 1\n",
   "    assert worker.queue.pop()[\"tries\"] == 1\n\n\ndef test_gives_up(monkeypatch):\n    monkeypatch.setattr(worker.settings, \"retries\", 1)\n    worker.queue.push({\"url\": \"u\", \"payload\": {}})\n    monkeypatch.setattr(worker, \"post\", lambda *a: (_ for _ in ()).throw(OSError()))\n    assert worker.run_once() is False\n    assert len(worker.queue) == 0\n")
PY
git commit -qam "test giving up"
