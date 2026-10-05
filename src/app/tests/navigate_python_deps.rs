use super::*;

#[test]
fn a_dependency_hands_a_name_on_through_its_imports() {
    let std = external_root(
        "py-deps-std",
        &[
            ("json/__init__.py", "def dumps(obj):\n    pass\n"),
            (
                "io.py",
                "from _io import (\n    BytesIO,\n    StringIO,\n)\n",
            ),
            ("collections/__init__.py", ""),
            ("collections/abc.py", "from _collections_abc import *\n"),
            ("_collections_abc.py", "class Sequence:\n    pass\n"),
            (
                "site-packages/pip/_internal/cli/cmdoptions.py",
                "json = object()\n",
            ),
        ],
    );
    let site = external_root(
        "py-deps-site",
        &[
            (
                "pytest/__init__.py",
                "from _pytest.fixtures import fixture\nfrom _pytest.mark import MARK_GEN as mark\n",
            ),
            ("_pytest/__init__.py", ""),
            ("_pytest/fixtures.py", "def fixture(fn=None):\n    pass\n"),
            (
                "_pytest/mark/__init__.py",
                "from .structures import MARK_GEN\n",
            ),
            (
                "_pytest/mark/structures.py",
                "class MarkGenerator:\n    pass\n\n\nMARK_GEN = MarkGenerator()\n",
            ),
            (
                "otherlib/tools.py",
                "def mark(msg):\n    pass\n\n\ndef fixture(fn):\n    pass\n",
            ),
            ("kombu/utils/json.py", "def dumps(value):\n    pass\n"),
            ("rest_framework/__init__.py", ""),
            (
                "rest_framework/serializers.py",
                "from rest_framework.exceptions import ValidationError\n",
            ),
            (
                "rest_framework/exceptions.py",
                "class ValidationError(Exception):\n    pass\n",
            ),
            ("lazy/__init__.py", "def __getattr__(name):\n    pass\n"),
            ("lazyreal.py", "def thing():\n    pass\n"),
            ("elsewhere/thing.py", "def thing():\n    pass\n"),
        ],
    );
    let (dir, mut a) = project_app(
        "py-deps",
        &[(
            "app/reexports.py",
            "import json\nimport pytest\nfrom collections.abc import Sequence\nfrom io import StringIO\nfrom rest_framework import serializers\nimport lazy\n\n\n@pytest.fixture\ndef thing(s: Sequence[int]):\n    return StringIO()\n\n\n@pytest.mark.skip\ndef test_x():\n    json.dumps({})\n    raise serializers.ValidationError(\"x\")\n    lazy.thing()\n",
        )],
    );
    use_roots(&mut a, Kind::Python, &[std.clone(), site.clone()]);
    let at = |root: &Path, file: &str, line: usize| format!("{}:{line}", root.join(file).display());
    for (code, want) in [
        (
            "pytest.fixture",
            jump(
                "fixture: via import _pytest.fixtures",
                &at(&site, "_pytest/fixtures.py", 1),
            ),
        ),
        (
            "pytest.mark",
            jump(
                "mark: via import _pytest.mark.structures",
                &at(&site, "_pytest/mark/structures.py", 5),
            ),
        ),
        (
            "s: Sequence",
            jump(
                "Sequence: via import _collections_abc",
                &at(&std, "_collections_abc.py", 1),
            ),
        ),
        (
            "return StringIO",
            jump("StringIO: via import io", &at(&std, "io.py", 3)),
        ),
        (
            "json.dumps",
            jump("dumps: via import json", &at(&std, "json/__init__.py", 1)),
        ),
        (
            "serializers.ValidationError",
            jump(
                "ValidationError: via import rest_framework.exceptions",
                &at(&site, "rest_framework/exceptions.py", 1),
            ),
        ),
    ] {
        d_on(&mut a, "app/reexports.py", code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    d_on(&mut a, "app/reexports.py", "lazy.thing");
    assert_eq!(
        shown(&mut a),
        Shown::Picker(
            "thing: by name, 2 declarations".into(),
            vec![
                (
                    "thing".into(),
                    "by name".into(),
                    "elsewhere/thing.py:1".into()
                ),
                ("thing".into(), "by name".into(), "lazyreal.py:1".into()),
            ]
        )
    );
    for d in [dir, std, site] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_root_inside_another_shows_each_declaration_once() {
    let std = external_root(
        "py-nested-std",
        &[("site-packages/foo/__init__.py", "def bar():\n    pass\n")],
    );
    let (dir, mut a) = project_app(
        "py-nested",
        &[("app/x.py", "from foo import bar\n\nbar()\n")],
    );
    use_roots(
        &mut a,
        Kind::Python,
        &[std.clone(), std.join("site-packages")],
    );
    d_on(&mut a, "app/x.py", "^bar");
    assert_eq!(
        shown(&mut a),
        jump(
            "bar: via import foo",
            &format!("{}:1", std.join("site-packages/foo/__init__.py").display())
        )
    );
    for d in [dir, std] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_class_declared_in_a_dependency_is_read() {
    let std = external_root(
        "py-classes-std",
        &[
            ("unittest/__init__.py", "from .case import TestCase\n"),
            (
                "unittest/case.py",
                "class TestCase(object):\n    def assertEqual(self, first, second, msg=None):\n        pass\n",
            ),
            (
                "pathlib.py",
                "class Path:\n    def write_bytes(self, data):\n        pass\n",
            ),
        ],
    );
    let site = external_root(
        "py-classes-site",
        &[
            (
                "django_softdelete/models.py",
                "class SoftDeleteModel:\n    objects = None\n",
            ),
            (
                "anyio/fileio.py",
                "class AsyncPath:\n    async def write_bytes(self, data):\n        pass\n",
            ),
            (
                "django/test/__init__.py",
                "from django.test.testcases import TestCase\n",
            ),
            (
                "django/test/testcases.py",
                "import unittest\n\n\nclass SimpleTestCase(unittest.TestCase):\n    client = None\n\n\nclass TestCase(SimpleTestCase):\n    pass\n",
            ),
            (
                "django/db/models/__init__.py",
                "from django.db.models.base import Model\n",
            ),
            (
                "django/db/models/base.py",
                "class ModelBase(type):\n    pass\n\n\nclass Model(metaclass=ModelBase):\n    pass\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "py-classes",
        &[
            (
                "app/deps.py",
                "import unittest\nfrom pathlib import Path\n\nfrom django_softdelete.models import SoftDeleteModel\n\n\nclass Document(SoftDeleteModel):\n    pass\n\n\nclass T(unittest.TestCase):\n    def test(self) -> None:\n        self.assertEqual(1, 1)\n\n\ndef use(p: Path) -> None:\n    p.write_bytes(b\"\")\n    Document.objects.all()\n",
            ),
            (
                "app/views.py",
                "from django.db import models\nfrom django.test import TestCase\n\n\nclass User(models.Model):\n    pass\n\n\nclass TestViews(TestCase):\n    def test(self):\n        self.client.get(\"/\")\n        self.assertEqual(1, 1)\n        User.objects.all()\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Python, &[std.clone(), site.clone()]);
    let at = |root: &Path, file: &str, line: usize| format!("{}:{line}", root.join(file).display());
    for (file, code, want) in [
        (
            "app/deps.py",
            "self.assertEqual",
            jump(
                "assertEqual \u{2192} TestCase.assertEqual (via self: T)",
                &at(&std, "unittest/case.py", 2),
            ),
        ),
        (
            "app/deps.py",
            "p.write_bytes",
            jump(
                "write_bytes \u{2192} Path.write_bytes (via p: Path)",
                &at(&std, "pathlib.py", 2),
            ),
        ),
        (
            "app/deps.py",
            "Document.objects",
            jump(
                "objects \u{2192} SoftDeleteModel.objects (via Document)",
                &at(&site, "django_softdelete/models.py", 2),
            ),
        ),
        (
            "app/views.py",
            "self.client",
            jump(
                "client \u{2192} SimpleTestCase.client (via self: TestViews)",
                &at(&site, "django/test/testcases.py", 5),
            ),
        ),
        (
            "app/views.py",
            "self.assertEqual",
            jump(
                "assertEqual \u{2192} TestCase.assertEqual (via self: TestViews)",
                &at(&std, "unittest/case.py", 2),
            ),
        ),
        (
            "app/views.py",
            "User.objects",
            jump("no definition for objects", "app/views.py:13"),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{file}: {code}");
    }
    for d in [dir, std, site] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn the_walk_outside_keeps_its_limits() {
    let std = external_root(
        "py-limits-std",
        &[
            ("implx.py", "def thing():\n    pass\n"),
            (
                "pkgx/__init__.py",
                "from implx import thing\nfor thing in range(1):\n    pass\n",
            ),
            ("cyc_a/__init__.py", "from cyc_b import cyx\n"),
            ("cyc_b/__init__.py", "from cyc_a import cyx\n"),
            (
                "twin/__init__.py",
                "try:\n    from fastimpl import twinned\nexcept ImportError:\n    from slowimpl import twinned\n",
            ),
            ("fastimpl.py", "def twinned():\n    pass\n"),
            ("slowimpl.py", "def twinned():\n    pass\n"),
            ("rel/__init__.py", ""),
            ("rel/mid.py", "from .deep import deeply\n"),
            ("rel/deep.py", "def deeply():\n    pass\n"),
            ("extlib/__init__.py", "VERSION = 1\n"),
            (
                "pathlibx.py",
                "import sys\nif sys.version_info >= (3, 12):\n    class PathX:\n        def write_all(self, data):\n            pass\nelse:\n    class PathX:\n        def write_all(self, data):\n            pass\n",
            ),
            ("pkgmark/__init__.py", "mark = object()\n"),
            ("dep/__init__.py", ""),
            (
                "dep/base.py",
                "from helpers import Base\n\n\nclass Thing(Base):\n    pass\n",
            ),
            (
                "helpers.py",
                "class Base:\n    def ping(self):\n        pass\n",
            ),
            ("relbase/__init__.py", ""),
            (
                "relbase/core.py",
                "from .parent import Parent\n\n\nclass Child(Parent):\n    pass\n",
            ),
            (
                "relbase/parent.py",
                "class Parent:\n    def hello(self):\n        pass\n",
            ),
            (
                "eggs/foo-1.0-py3.11.egg/eggfoo/__init__.py",
                "def bar():\n    pass\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "py-limits",
        &[
            (
                "helpers.py",
                "class Base:\n    def ping(self):\n        pass\n",
            ),
            ("app/util.py", "def helper():\n    pass\n"),
            (
                "app/limits.py",
                "import pkgx\nimport cyc_a\nimport twin\nfrom rel import mid\nfrom extlib import helper\nfrom pathlibx import PathX\nfrom pkgmark import mark\nfrom dep.base import Thing\nfrom relbase.core import Child\nfrom eggfoo import bar\n\n\ndef use(p: PathX, m: mark, t: Thing, c: Child):\n    pkgx.thing()\n    cyc_a.cyx()\n    twin.twinned()\n    mid.deeply()\n    helper()\n    p.write_all(b\"\")\n    m.skip()\n    t.ping()\n    c.hello()\n    bar()\n",
            ),
        ],
    );
    let egg = std.join("eggs/foo-1.0-py3.11.egg");
    use_roots(&mut a, Kind::Python, &[std.clone(), egg.clone()]);
    let at = |root: &Path, file: &str, line: usize| format!("{}:{line}", root.join(file).display());
    let file = "app/limits.py";
    for (code, want) in [
        (
            "pkgx.thing",
            Shown::Picker(
                "thing: by name, 1 match".into(),
                vec![("thing".into(), "by name".into(), "implx.py:1".into())],
            ),
        ),
        (
            "cyc_a.cyx",
            jump("no definition for cyx", "app/limits.py:15"),
        ),
        (
            "twin.twinned",
            Shown::Picker(
                "twinned: 2 declarations".into(),
                vec![
                    (
                        "twinned".into(),
                        "via import fastimpl".into(),
                        "fastimpl.py:1".into(),
                    ),
                    (
                        "twinned".into(),
                        "via import slowimpl".into(),
                        "slowimpl.py:1".into(),
                    ),
                ],
            ),
        ),
        (
            "mid.deeply",
            jump("deeply: via import rel.deep", &at(&std, "rel/deep.py", 1)),
        ),
        (
            "^    t.ping",
            jump(
                "ping \u{2192} Base.ping (via t: Thing)",
                &at(&std, "helpers.py", 2),
            ),
        ),
        (
            "c.hello",
            jump(
                "hello \u{2192} Parent.hello (via c: Child)",
                &at(&std, "relbase/parent.py", 2),
            ),
        ),
        (
            "^    bar",
            jump("bar: via import eggfoo", &at(&egg, "eggfoo/__init__.py", 1)),
        ),
    ] {
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{code}");
    }
    d_on(&mut a, file, "^    helper");
    let Shown::Jump(_, place) = shown(&mut a) else {
        panic!("a jump");
    };
    assert_eq!(place, "app/util.py:1");
    for code in ["p.write_all", "m.skip"] {
        d_on(&mut a, file, code);
        let message = a.message.clone();
        shown(&mut a);
        assert!(!message.contains("(via "), "{code}: {message}");
    }
    for d in [dir, std] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn the_first_typed_d_walks_the_venv() {
    let base = external_root(
        "py-first-base",
        &[
            ("bin/python3.99", ""),
            (
                "lib/python3.99/unittest/__init__.py",
                "from .case import TestCase\n",
            ),
            (
                "lib/python3.99/unittest/case.py",
                "class TestCase(object):\n    def assertEqual(self, first, second, msg=None):\n        pass\n",
            ),
        ],
    );
    let cfg = format!("home = {}\n", base.join("bin").display());
    let (dir, mut a) = project_app(
        "py-first",
        &[
            (".venv/pyvenv.cfg", cfg.as_str()),
            (".venv/lib/python3.99/site-packages/.keep", ""),
            (
                "app/t.py",
                "import unittest\n\n\nclass T(unittest.TestCase):\n    def test(self):\n        self.assertEqual(1, 1)\n",
            ),
        ],
    );
    d_on(&mut a, "app/t.py", "self.assertEqual");
    let case = base
        .canonicalize()
        .unwrap()
        .join("lib/python3.99/unittest/case.py");
    assert_eq!(
        shown(&mut a),
        jump(
            "assertEqual \u{2192} TestCase.assertEqual (via self: T)",
            &format!("{}:2", case.display())
        )
    );
    for d in [dir, base] {
        std::fs::remove_dir_all(d).unwrap();
    }
}

#[test]
fn a_member_outside_is_looked_for_in_the_imported_packages_first() {
    let site = external_root(
        "py-reach-site",
        &[
            (
                "requests/models.py",
                "class Response:\n    def close(self):\n        pass\n\n    def json(self):\n        pass\n",
            ),
            (
                "requests/sessions.py",
                "class Session:\n    def close(self):\n        pass\n",
            ),
            (
                "otherlib/files.py",
                "class Handle:\n    def close(self):\n        pass\n\n    def json(self):\n        pass\n",
            ),
        ],
    );
    let (dir, mut a) = project_app(
        "py-reach",
        &[(
            "app/x.py",
            "import requests\n\n\ndef fetch(url):\n    r = requests.get(url)\n    r.close()\n    r.json()\n",
        )],
    );
    a.no_external();
    use_roots(&mut a, Kind::Python, std::slice::from_ref(&site));
    d_on(&mut a, "app/x.py", "    r.|close");
    assert_eq!(
        shown(&mut a),
        picker(
            "close: by name, 2 declarations",
            &[
                ("Response.close", "requests/models.py:2"),
                ("Session.close", "requests/sessions.py:2"),
            ]
        ),
        "the imported package declares it twice: no namesake of another package is offered"
    );
    d_on(&mut a, "app/x.py", "    r.|json");
    assert_eq!(
        shown(&mut a),
        picker(
            "json: by name, 2 declarations",
            &[
                ("Handle.json", "otherlib/files.py:5"),
                ("Response.json", "requests/models.py:5"),
            ]
        ),
        "one declaration in the imported package is no proof: every package is read, as before"
    );
    for d in [dir, site] {
        std::fs::remove_dir_all(d).unwrap();
    }
}
