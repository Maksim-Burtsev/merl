//! `d` in Python on the names a dependency outside the project declares (#329).

use super::*;

/// #329. A dependency's module that hands a name on through its own imports is followed, as the
/// project's are: a named import, an alias, a `*`, a relative import, down to a compiled source,
/// where the import line is the answer. A module is matched from the root it lies under, so
/// `json` is never a `json.py` deeper in another package.
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
            // A name made up by a module `__getattr__` is not followed.
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
    // Nothing followed: found everywhere by name, and offered, never jumped to.
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

/// #329. A root inside another, as `sys.path` lists `lib/python3.11` and its `site-packages`, is
/// walked once: a declaration there is one row, named from the inner root.
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

/// #340. A class declared in a dependency is read as the project's are: a receiver typed or
/// built as it, a base of a project class, the members it declares and those of its own bases,
/// through its file's imports, relative ones too. An attribute no line declares, such as the
/// `objects` Django's metaclass makes, stays unknown.
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
