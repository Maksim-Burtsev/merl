# `d` on Python: misses in paperless-ngx against pyright

The bench has 270 cursors. It scored 223: ok 155 · pick-hit 26 · WRONG 5 · pick-miss 17 · none 20. It did not score 47: 28 oracle-none/picker, 11 on-decl, 5 oracle-none/jump and 3 oracle-none/stay.

I read every WRONG, none and pick-miss row, and 15 of the 26 pick-hit rows. The other 11 pick-hits repeat a cause already read:
- 0035 is like 0023, 0150 like 0075, 0173 like 0058.
- 0181 and 0206 are like 0141, 0190 like 0019.
- 0196 and 0213 are like 0112, 0257 like 0115, 0262 like 0084.
- 0260 is an `@overload` set that the oracle lists in full.

Repros are in `d-bench/work-python/repro/`: a project whose `.venv` links to paperless's. The cursors are in `repro*-in.tsv`, the results in `repro*-out.tsv`.

## Corrected tally (57 rows read)

| bench verdict | read | artifacts | real |
|---|---|---|---|
| WRONG | 5 | 2. 0212 and 0250: the parameter sits on its own line of a wrapped `def`, and merl lands on the `def` line 2–3 lines above it. | 3 |
| none | 20 | 3. 0128 `RuntimeError`, 0159 `ZeroDivisionError` and 0259 `ValueError` are builtins whose only source is pyright's bundled typeshed, so "no definition" is right. | 17 |
| pick-miss | 17 | 1. 0170 is `f.read()` on `with path.open("rb") as f`: the target is compiled `_io`, which only typeshed has. | 16 |
| pick-hit | 15 | 2. 0023: the oracle returns the same two assignments. 0216: the scorer's module key counted `pathlib.parents` as a hit for `mkdir(parents=…)`, so it is really a pick-miss. | 13 |

After the moves:

| verdict | rows |
|---|---|
| ok | 2 |
| none, and none is right | 3 |
| pick-hit | 14 |
| WRONG | 3 |
| pick-miss | 17 |
| none | 17 |
| no target on disk | 1 |

Artifact share by bench verdict: WRONG 40 %, none 15 %, pick-miss 6 %, pick-hit 13 %.

Three pick-misses also have no source to land on: 0133 `next`, 0137 `map` and 0111 `str.replace`. They still count as real misses, because the honest answer is `builtin`, not a picker of 33–98 namesakes.

## Causes, most harmful first

The count is the number of the 51 real misses I read whose main cause is this one. The 51 are 3 WRONG, 17 pick-miss (pm), 17 none and 14 pick-hit (ph). Some causes only add to another row's miss; these show as "(secondary)". The letters are reused in the issues below.

| cause | count | WRONG? | example: file:line, code → merl → should be | fix sketch (rules over lines) | risk | size |
|---|---|---|---|---|---|---|
| **A.** A word in the module path of an import line has no rule. The word after `.` is read as a member of a value and searched by name. | 10 (1 WRONG, 4 none, 5 pm) | yes | 0021 `test_highlight_query_guard.py:32` `from paperless_testing.factories import DocumentFactory`, cursor on `factories` → `faker/proxy.py:294 def factories` (by name, 1 match) → `src/paperless_testing/factories.py:1`. 0016 `from documents.search._errors import …`, cursor on `_errors` → none. | On `from P import …` or `import P [as x]`, a word inside P names the module `P[..=i]`. Look it up with `module_files` in the project (leading dots count from the file's package), else in the external roots, anchored as in I. Land on line 1 of `__init__.py`, `.py` or `.pyi`, as #280's `Reason::Module` does. If nothing is found, say `no module P`; never search by name. | none: import lines only | S |
| **B.** A module that an import binds, outside the project, used as a qualifier. #280 lands on a project module only. Outside, merl looks for a declaration named like the module, then searches everywhere by name. | 3 (1 WRONG, 2 none) | yes | 0217 `test_api_workflows.py:877` `json.dumps(` under `import json`, cursor on `json` → `…/python3.11/site-packages/pip/_internal/cli/cmdoptions.py:1187 json: Callable[..., Option] = partial(` → `json/__init__.py:1`. 0031 `views.py:834` `serializers.ListField(` (from `from rest_framework import serializers`) → none → `rest_framework/serializers.py:1`. | Run #280's `whole_module` outside too: when the bound path resolves in full (anchored) to a module or package, land on its first line, unless the package above declares or hands on the name. A plain `import x` binds a module, so its name is never searched by name. | low | S |
| **C.** Classes declared outside the project are not read, so a proven type or a base from a dependency ends the typed walk. Three shapes (C1–C3 in the example column). | 21 (1 WRONG, 10 none, 6 pm, 4 ph) | yes, plus latent: repro r20 | **C1**, a member inherited from an outside base: 0063 `test_api_app_config.py:1219` `self.assertEqual(` → none → `unittest/case.py:868`. 0054 `self.client.get(` → picker of two unrelated project `client`s. **C2**, the receiver's type is outside: 0248 `test_bulk_edit.py:1581` `mock_group.return_value…` with `mock_group: mock.Mock` → `anyio/_core/_tasks.py:359 def return_value` (by name, 1 match) → `unittest/mock.py:557`. **C3**, a class attribute through an outside base: 0033 `test_consumer.py:473` `Document.objects.first()` → none → `django_softdelete/models.py:49 objects = SoftDeleteManager()`. | (i) Guard, S. Take `self.x`, or `x.y` on a proven project type, whose project ancestry is read to the end and has a base imported from outside. Drop the project namesakes of classes outside that ancestry and say `no definition for client (inherited from TestCase)`. (ii) The real fix, L. `declaration()` resolves an import outside to its `class` line (anchored, with re-exports followed as in G). `members_of`, `field_of` and `above` read that external file. An outside class's bases resolve through that file's own imports. The rules stay: declared once, bases that agree, only what needs no MRO. | (i) none. (ii) moderate. Attributes a metaclass makes (`objects` on a plain `models.Model`) stay none. Class bodies under `if TYPE_CHECKING:`. Big files. | S + L |
| **H.** A member found by name jumps when it has one candidate outside the project, although fields outside are never collected. | secondary in 2 WRONG (0021, 0248) | yes | 0248 above: anyio's is the only *method* named `return_value` outside. `mock.py`'s `return_value = property(…)` is a field, so it never becomes a candidate. | For a member of a value of unknown type whose every candidate is outside the project, offer the candidate and never jump, as a cut search already does. | costs 2 ok here: 0068 `captured_queries` and 0136 `at_level` become 1-row pickers | S |
| **G.** Re-exports outside the project are not followed. When a dependency's module binds the name through an import, merl falls back to every external file, by name, at the top level. | 7 (2 pm, 5 ph); also the fallback behind 0217 | no here; yes when there is one namesake | 0201 `test_tesseract_parser.py:1214` `@pytest.mark.parametrize(`, cursor on `mark` → 3 × torch `def mark` → `_pytest/mark/structures.py:639 MARK_GEN = …` (via `pytest/__init__.py:39 from _pytest.mark import MARK_GEN as mark`). 0255 `serialisers.py:1880` `serializers.ValidationError` → 12 by name → `rest_framework/exceptions.py:143`. | Run `handed_on` (#100) on external modules too: `from x import y [as z]` and `from x import *`, four deep, with relative imports read against the dependency's own package. A compiled source (`io.py: from _io import … StringIO`) lands on that binding line. Drop the everywhere-by-name fallback for an imported name, or at least never jump on it. | low. Two sources (try/except ImportError) make a picker, as in the project. | M |
| **D.** The stdlib root `…/lib/python3.11` also walks the base interpreter's `site-packages`: pip, setuptools and `pip._vendor`, 727 files. The venv excludes it (`include-system-site-packages = false`). | secondary in 1 WRONG (0217's target); pip/setuptools rows in 26 of 270 results | yes | repro r22: `requests.get` → picker of `pip/_vendor/requests/api.py:74`, listed first, and `requests/api.py:62` | Skip `site-packages` and `dist-packages` at depth 1 of the stdlib root, unless pyvenv.cfg says `include-system-site-packages = true`. | none | S |
| **E.** A class-qualified word behind an outside import, such as `User.objects`, is searched as a top-level name of that module, then of any module. | secondary in 1 pm (0081) | no here; yes when there is one namesake | 0081 `test_api_objects.py:797` `User.objects.create(` → `nltk/misc/chomsky.py:96 objects = """…` and `twisted/test/test_sob.py:19 objects = [` → `django/contrib/auth/models.py:486 objects = UserManager()` | When the import's path resolves to a module shorter than the path, `User` is a name in that module. Look for `User.objects` there with `search::qualified`, and never for top-level namesakes elsewhere. Walking the class itself is C. | none | S |
| **F.** Builtins have no source file. Yet a bare builtin (`next`, `map`) and a member of a proven builtin type (`str.replace`) are searched by name in every external file. | 3 pm (+3 nones that are already honest) | no | 0133 `test_api_objects.py:172` `next(r for r in …)` → 51 methods named `next`. 0111 `file_handling.py:135` `rendered_filename.replace(`, with the value from a `-> str \| None` function → 98 `replace`s. | Take a bare word that no scope, import or `*`-import binds, that no project module declares at its top level, and that is in `dir(builtins)`: say `next: builtin, no source` and run no grep. A receiver typed as `str`/`bytes`/`int`/`float`/`bool`/`list`/`dict`/`set`/`tuple`/`object` gets `replace → str.replace (builtin, no source)`. A bare unbound word that is not a builtin is searched outside only at the top level of `*`-imported modules, never among methods. | none | S |
| **I.** An outside module is matched anywhere in the path, not from its root: `in_module` accepts the parts in order anywhere. | 1 ph (+0150) | no | 0075 `test_api_custom_fields.py:129` `json.dumps(` → `json/__init__.py:183` plus `django_extensions/db/fields/json.py:21`, `…/mongodb/fields/json.py:32`, `kombu/utils/json.py:54` and `rest_framework/utils/json.py:23` → `json/__init__.py:183` only | For Python, the path relative to the root it lies under must start with the module parts (`json/…`, `json.py`, `json.pyi`), matched case-sensitively. | low | S |
| **K.** A keyword argument names a parameter of the callee, and no rule reads it: the result is none, or a picker of namesake methods. | 2 (1 none, 1 pm) | no | 0074 `test_backend.py:340` `backend.search_ids("monthly", user=None, search_mode=…)` → none → `_backend.py:882 search_mode: SearchMode = …`. 0216 `existing.parent.mkdir(parents=True)` → 12 `parents` → `pathlib.py:1111` | Scan back to the unmatched `(` and take the callee (`name` or `a.b.name`). Resolve it as `d` does. If that gives one `def`, land on the parameter's line in its parameter list, wrapped or not. For a class, `**kwargs` or several defs, keep today's behaviour, minus the grep outside: a parameter is never declared outside. | low | M |
| **J.** A nested `def` (a closure) is not taken as the local binding, because `names_itself` drops `def` and `class` lines from the scope's bindings. | 1 ph | no | 0002 `base_model.py:260` `_choice(` → picker of `base_model.py:222` (this function's closure) and `ai_classifier.py:258` → 222 | Keep a `def` or `class` line that an enclosing *function* binds as a `Local` answer. | low | S |
| **P.** A parameter on its own line of a wrapped `def` lands on the `def` line, because `python_params` returns `d + 1`. | 2 near-hits (scored WRONG by the bench) | bench only | 0250 `test_query_negation.py:54` `matched_ids(` → `:27 def test_…(` → `:30 matched_ids: Callable[…],` | Land on the line where the parameter sits in the list. | none | S |
| Beyond rules | 2 ph + 0170 | no | 0044 `logger.info`, with `logger = logging.getLogger(…)`, which is untyped in the stdlib source. 0189 `.year` of `timezone.localtime(…)`, also untyped. 0170 `f.read()`, compiled `_io`. | By name is the right answer. The picker could rank the factory module's rows first (`logging/__init__.py:1479` on top) without narrowing. | none | S |

## Pick-hits: why a picker, and what would narrow it

| rows | word | why a picker | what narrows it |
|---|---|---|---|
| 0002 | `_choice` | The closure is not taken as the local binding (J). | J makes it a jump. |
| 0095, 0112, 0255 | `timezone.datetime`, `pytest.param`, `serializers.ValidationError` | A re-export inside a dependency (G). | G makes it a jump. |
| 0019 | `Sequence` | `collections/abc.py: from _collections_abc import *` is not followed (G). | G leads to `_collections_abc.py:973`, the runtime class. The oracle says `typing.py:2754`, so the bench would score G's answer WRONG. |
| 0115 | `pytest.fixture` | G, then 3 `@overload`s plus the implementation. | G gives the oracle's own 3. Optionally, jump to the undecorated implementation (S). |
| 0075 | `json.dumps` | The module is matched unanchored (I). | I makes it a jump. |
| 0084, 0058, 0139 | `Path(x).stat`, `spy.assert_not_called`, `patcher.stop` | The type is proven or provable but declared outside (C2). `mock.patch()` constructs `_patch(…)` in every `return`. | C. For 0058 also: `mock.py:215` is a `def` nested in a function, not a method, so a member search should skip it (S). |
| 0141 | `self.client.post` | `client` is inherited from `APITestCase` (C1). | C. The runtime answer is `rest_framework/test.py:300 APIClient.post`, since `client_class = APIClient`, so the oracle's `django/test/client.py:1141` is debatable. |
| 0044, 0189 | `logger.info`, `.year` | The value comes from an untyped factory. | Nothing sound narrows it; rank the factory module's rows first. |
| 0023 | `ids` | Two assignments in the function; the oracle lists both. | Nothing to do. |
| 0216 | `parents=` | A keyword argument (K), counted as a hit only through the module key. | K. |

## Latency

- **Every slow row searched the whole outside.** 88 of 270 rows took over 300 ms; p50 was 40 ms, p75 658 ms, p90 755 ms and the max 3524 ms. Each of the 88 grepped every external file: 23.4k `.py` and `.pyi` files, about 300 MB. torch, transformers, sympy and scipy alone are about 7.4k of those files.
  - Warm, one such grep takes 0.6–0.7 s.
  - The first full greps of a session take 3.5–3.9 s on a cold page cache. 0010 was the first in the run at 3524 ms; when I reran 0044 it took 3762 ms, then 714 ms.
  - The first lookup outside the project also walks the roots, about 0.9 s (row 0008).
- **Which lookups are slow.** The 88 split into 55 member lookups by name (unknown receiver or broken chain), 18 lookups that searched outside and found nothing, 14 bare or imported words by name, and 1 via import. Lookups that an import narrows stay fast: 36 of 37 `via import` lookups outside the project took under 300 ms, and so did all 78 inside it.
- **What the fixes would cut.** A, B, F, G and K replace the full grep with a read of a few files, or no grep at all, for 36 of the 88 slow rows (10 + 3 + 6 + 12 + 5). The 5 K rows are keyword arguments, which are grepped outside for nothing today. D removes 727 files, 3 % of the grep. What stays slow are members of values whose type is unknown, until C proves the type.

## Top improvements

### 1. `d` on a module name lands on the module: import-line paths, and modules outside the project (A, B)
- Words in `from a.b.c import X` or `import a.b.c` have no rule. `d` on `c` reads a member of a value and searches by name: 10 of the 57 misses read, one of them a wrong jump.
- A word inside an import's module path names the module `parts[..=i]`. Look it up with `module_files` in the project (leading dots from the file's package), else in the external roots, anchored as in #2. Land on line 1 of `__init__.py`, `.py` or `.pyi`, with status `utils: module src/documents/utils.py`. If nothing is found, say `no module documents.utils`; never search by name.
- A name that an import binds to a module outside (`import json`, `from rest_framework import serializers`) lands on that module's first line, as #280 does inside the project, unless the package above declares or hands on the name.
- Bench: 13 misses become jumps, 2 of them WRONG today.

```python
# app/repos.py: class UserRepo: ...
from app.repos import UserRepo                   # d on `repos`: today → fsspec GithubFileSystem.repos (by name, 1 match); want app/repos.py:1
from django.core.management import CommandError  # d on `management`: today "no definition"; want django/core/management/__init__.py:1
import json
json.dumps({})                                   # d on `json`: today → pip/_internal/cli/cmdoptions.py:1187; want json/__init__.py:1
```

### 2. Outside the project, `d` follows re-exports and finds a module from its root (G, I, D)
- `handed_on` (#100) follows a module's own imports only inside the project. Outside, a module that binds the name through an import falls back to every external file by name: `pytest.mark` gives three torch `def mark`, none of them right. Run the same walk on dependency modules: `as`, `*`, relative imports against the dependency's own package, four deep. A compiled source (`io.py: from _io import … StringIO`) lands on that binding line.
- `in_module` matches the parts anywhere in the path, so `json` also matches `kombu/utils/json.py`, and `requests` also matches `pip/_vendor/requests`. For Python, the path relative to its root must start with the module parts.
- The stdlib root includes the base interpreter's `site-packages`: pip and setuptools, 727 files. Skip it unless pyvenv.cfg says `include-system-site-packages = true`.
- Bench: 7 read misses become jumps or the oracle's own list: 0019, 0075, 0095, 0112, 0115, 0201 and 0255. So do 5 unread duplicates.

```python
import json
import pytest
import requests

@pytest.fixture                     # d on `fixture`: today 8 by name (joblib, pytest_asyncio…); want _pytest/fixtures.py (3 overloads + impl)
def thing(): ...

@pytest.mark.parametrize("x", [1])  # d on `mark`: today 3 × torch `def mark`; want _pytest/mark/structures.py:639
def test_x(x):
    json.dumps({})                  # d on `dumps`: today 5 declarations; want json/__init__.py:183
    requests.get("x")               # d on `get`: today pip/_vendor/requests/api.py:74 + requests/api.py:62; want the latter
```

### 3. A member inherited from a dependency's class never lands on an unrelated project class (C guard, E, H)
- **`self.client` in a Django `TestCase` subclass.** The class and its project bases are read to the end, and the rest of the ancestry is outside, so no other project class's `client` can be the answer. Yet `d` offers those namesakes, and jumps when there is only one. Drop the project namesakes outside that ancestry and say `no definition for client (inherited from TestCase)`, until #4 finds the real one.
- **`User.objects`, with `User` imported from a dependency.** Today it is searched as a top-level `objects`, first of that module, then of any module (nltk, twisted). The path resolved short of `User`, so `User` is a name: look for `User.objects` in that module only.
- **One candidate found outside the project.** For a member of a value of unknown type whose candidates are all outside, offer the candidate rather than jump. Fields outside are never collected, so `1 match` proves nothing there.
- Bench:
  - removes 1 WRONG (0248);
  - turns 3 pick-misses into honest nones (0054, 0235, 0081);
  - costs 2 ok rows, which become 1-row pickers (0068, 0136).

```python
# app/mailer.py
class Mailer:
    def __init__(self, client):
        self.client = client
# app/test_views.py
from django.test import TestCase
class TestViews(TestCase):
    def test_get(self) -> None:
        self.client.get("/")      # today: client → Mailer.client (by name, 1 match)
# app/cases.py
from unittest import mock
def use(m: mock.Mock) -> None:
    m.return_value = None         # today: → anyio TaskHandle.return_value (by name, 1 match)
```

### 4. `d` reads classes declared outside the project: proven types, inherited members and class attributes (C)
- `declaration()` finds nothing for a type imported from a dependency. So each of these ends in the search by name:
  - a proven `p: Path` or `m: mock.Mock`;
  - `Path(x).stat()`;
  - `patcher = mock.patch(…)`, although every `return` constructs `_patch`;
  - every member inherited from `TestCase`, `BaseCommand` or `SoftDeleteModel`.
- That is 21 of the 57 misses read, the largest gap for Python. `self.assertEqual` in any `TestCase` says "no definition".
- The fix: resolve the import outside to its `class` line with #2's walk. Let `members_of`, `field_of` and `above` read that file. Resolve an outside class's bases through that file's own imports. Keep the existing rules: declared once, bases that agree, only what needs no MRO.
- This will not answer:
  - attributes a metaclass makes, such as `objects` on a plain `models.Model` subclass;
  - untyped factories such as `logging.getLogger`;
  - `with … as f`;
  - a field under `if TYPE_CHECKING:` in a class body.
- Size L, in two M steps: first the outside class line, then its bases.

```python
import unittest
from pathlib import Path
class T(unittest.TestCase):
    def test(self) -> None:
        self.assertEqual(1, 1)   # today: no definition; want unittest/case.py:868
def use(p: Path) -> None:
    p.write_bytes(b"")           # today: 10 by name; want pathlib.py:1061
```

### 5. Python builtins say `builtin` instead of opening a picker of namesakes (F)
- A bare word that no scope, import or `*`-import binds, and that no project module declares at its top level, can only be a builtin. Today `next(…)` opens 51 methods named `next`, after a 0.65 s grep.
  - For a name in `dir(builtins)`, say `next: builtin, no source` and run no grep.
  - Look for any other unbound bare word outside only at the top level of the `*`-imported modules, never among methods.
- A receiver typed as `str`, `bytes`, `int`, `float`, `bool`, `list`, `dict`, `set`, `tuple` or `object` gets `replace → str.replace (builtin, no source)`. Today it gets 98 `replace`s.
- Bench: 3 pick-misses become honest answers, and each saves one full external grep.

```python
def render() -> str | None: ...
def go() -> None:
    s = render()
    s.replace("a", "b")       # today: replace: by name, 98 declarations
    first = next(iter([1]))   # today: next: by name, 51 declarations
```

### Smaller follow-ups
- **K, keyword argument → the callee's parameter (M).** 0074 today says "no definition"; repro r18: `search_ids("x", search_mode=…)`.
- **J, a closure is the local binding (S).**
- **P, land on the parameter's own line (S).**
- **Member search skips a `def` nested in a function (S).** Example: `mock.py:215`.
- **`@overload` stubs plus an implementation (S).** Optionally, jump to the undecorated implementation.

## Bench caveats
- `collections.abc.Sequence` and `Callable` (0019, 0190) are scored against `typing.py`. Following the star import lands on `_collections_abc.py`, the runtime class, which the bench would call WRONG.
- In DRF's `APITestCase`, `self.client` is an `APIClient`. pyright answers Django's `Client` (0141, 0181, 0206).
- Builtins and `_io` exist only in pyright's bundled typeshed. The scorer's module key turned 0216 into a hit.
