#!/usr/bin/env python3
import pathlib
import re
import subprocess
import tempfile
import zlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
COMMIT = re.search(r'const COMMIT: &str = "(\w+)";', (ROOT / "src/typeshed.rs").read_text())[1]

with tempfile.TemporaryDirectory() as tmp:
    subprocess.run(["git", "init", "-q", tmp], check=True)
    git = ["git", "-C", tmp]
    subprocess.run(git + ["fetch", "-q", "--depth", "1", "https://github.com/python/typeshed", COMMIT], check=True)
    subprocess.run(git + ["checkout", "-q", "FETCH_HEAD", "--", "stdlib", "LICENSE"], check=True)
    stdlib = pathlib.Path(tmp, "stdlib")
    stubs = sorted(p for p in stdlib.rglob("*.pyi") if "@tests" not in p.parts)
    data = "".join(f"{p.relative_to(stdlib).as_posix()}\0{p.read_text()}\0" for p in stubs)
    (ROOT / "typeshed").mkdir(exist_ok=True)
    (ROOT / "typeshed/stdlib.zlib").write_bytes(zlib.compress(data.encode(), 9))
    (ROOT / "typeshed/LICENSE").write_text(pathlib.Path(tmp, "LICENSE").read_text())
    print(f"typeshed {COMMIT}: {len(stubs)} stubs, {len(data)} bytes")
