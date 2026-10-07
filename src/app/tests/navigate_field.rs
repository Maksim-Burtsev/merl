use super::*;

#[test]
fn a_field_is_a_target() {
    let namesakes = |word: &str, row: (&str, &str)| {
        picker(
            &format!("{word}: at a declaration, 1 other by name"),
            &[row],
        )
    };
    let cases: Vec<(&str, &str, &str, &str, Shown)> = vec![
        (
            "Annotated in the class body, with a value and without",
            "python",
            "fields.py",
            "issue.poster_id",
            jump(
                "poster_id \u{2192} Issue.poster_id (via issue: Issue)",
                "fields.py:16",
            ),
        ),
        (
            "Annotated in the class body, with a value and without",
            "python",
            "fields.py",
            "issue.title",
            jump(
                "title \u{2192} Issue.title (via issue: Issue)",
                "fields.py:15",
            ),
        ),
        (
            "Assigned in the `__init__` of the base class, and again in a method of `Issue`",
            "python",
            "fields.py",
            "issue.audit",
            jump(
                "audit \u{2192} Base.audit (via issue: Issue)",
                "fields.py:8",
            ),
        ),
        (
            "A field of `Issue` before a method of `Base`",
            "python",
            "fields.py",
            "issue.summary",
            jump(
                "summary \u{2192} Issue.summary (via issue: Issue)",
                "fields.py:17",
            ),
        ),
        (
            "",
            "python",
            "fields.py",
            "await self.repo",
            jump("repo \u{2192} Issue.repo (via self: Issue)", "fields.py:21"),
        ),
        (
            "A later assignment is not the declaration",
            "python",
            "fields.py",
            "^            self.labels",
            jump(
                "labels \u{2192} Issue.labels (via self: Issue)",
                "fields.py:22",
            ),
        ),
        (
            "",
            "python",
            "chains.py",
            "self.uow.users",
            jump(
                "users \u{2192} UnitOfWork.users (via self.uow: UnitOfWork)",
                "chains.py:10",
            ),
        ),
        (
            "",
            "python",
            "fields.py",
            "comment.poster_id",
            picker(
                "poster_id: by name, 2 declarations",
                &[
                    ("Issue.poster_id", "fields.py:16"),
                    ("Comment.poster_id", "fields.py:40"),
                ],
            ),
        ),
        (
            "`body : str` in the docstring declares nothing",
            "python",
            "fields.py",
            "comment.body",
            jump(
                "body \u{2192} Comment.body (by name, 1 match)",
                "fields.py:41",
            ),
        ),
        (
            "`total: int = 0` is a local",
            "python",
            "fields.py",
            "comment.total",
            jump("no definition for total", "fields.py:52"),
        ),
        (
            "The first binding of `Point.offset` is a tuple target",
            "python",
            "fields.py",
            "p.offset",
            picker(
                "offset: by name, 2 declarations",
                &[
                    ("Point.offset", "fields.py:68"),
                    ("Cursor.offset", "fields.py:76"),
                ],
            ),
        ),
        (
            "",
            "python",
            "fields.py",
            "^    poster_id",
            namesakes("poster_id", ("Comment.poster_id", "fields.py:40")),
        ),
        (
            "",
            "python",
            "fields.py",
            "^        self.poster_id",
            namesakes("poster_id", ("Issue.poster_id", "fields.py:16")),
        ),
        (
            "The parameter handed on, not the field it is handed to, and named so (#100)",
            "python",
            "fields.py",
            "self.repo = repo",
            jump("repo \u{2192} Issue.__init__.repo (local)", "fields.py:19"),
        ),
        (
            "A class whose base is outside the project and that does not declare the field: it comes from outside or from a subclass, and no other class's namesake is it (#342). A nested class of its own is found as before",
            "python",
            "fields.py",
            "if self.poster_id",
            jump("no definition for poster_id", "fields.py:61"),
        ),
        (
            "A class whose base is outside the project and that does not declare the field: it comes from outside or from a subclass, and no other class's namesake is it (#342). A nested class of its own is found as before",
            "python",
            "fields.py",
            "self.Options",
            jump(
                "Options \u{2192} Encoder.Options (by name, 1 match)",
                "fields.py:57",
            ),
        ),
        (
            "",
            "typescript",
            "fields.ts",
            "issue.posterId",
            jump(
                "posterId \u{2192} Issue.posterId (via issue: Issue)",
                "fields.ts:9",
            ),
        ),
        (
            "",
            "typescript",
            "fields.ts",
            "issue.title",
            jump(
                "title \u{2192} Issue.title (via issue: Issue)",
                "fields.ts:8",
            ),
        ),
        (
            "`Issue.close` assigns it again",
            "typescript",
            "fields.ts",
            "issue.audit",
            jump(
                "audit \u{2192} Base.audit (via issue: Issue)",
                "fields.ts:4",
            ),
        ),
        (
            "A parameter of a constructor wrapped over several lines",
            "typescript",
            "fields.ts",
            "this.repo",
            jump("repo \u{2192} Issue.repo (via this: Issue)", "fields.ts:13"),
        ),
        (
            "",
            "typescript",
            "fields.ts",
            "this.title",
            jump(
                "title \u{2192} Issue.title (via this: Issue)",
                "fields.ts:8",
            ),
        ),
        (
            "Both sides of `this.close = this.close.bind(this)` are the method",
            "typescript",
            "fields.ts",
            "this.close",
            jump(
                "close \u{2192} Issue.close (via this: Issue)",
                "fields.ts:21",
            ),
        ),
        (
            "Both sides of `this.close = this.close.bind(this)` are the method",
            "typescript",
            "fields.ts",
            "= this.close",
            jump(
                "close \u{2192} Issue.close (via this: Issue)",
                "fields.ts:21",
            ),
        ),
        (
            "A `case` block is no object literal: `this` is still the class",
            "typescript",
            "fields.ts",
            "String(this.posterId",
            jump(
                "posterId \u{2192} Issue.posterId (via this: Issue)",
                "fields.ts:9",
            ),
        ),
        (
            "A `case` block is no object literal: `this` is still the class",
            "typescript",
            "fields.ts",
            "return this.title",
            jump(
                "title \u{2192} Issue.title (via this: Issue)",
                "fields.ts:8",
            ),
        ),
        (
            "But the literal a `case` returns is: its `this` is not the class",
            "typescript",
            "fields.ts",
            "`${this.posterId",
            picker(
                "posterId: by name, 2 declarations",
                &[
                    ("Issue.posterId", "fields.ts:9"),
                    ("Comment.posterId", "fields.ts:47"),
                ],
            ),
        ),
        (
            "",
            "typescript",
            "chains.ts",
            "this.uow.users",
            jump(
                "users \u{2192} UnitOfWork.users (via this.uow: UnitOfWork)",
                "chains.ts:4",
            ),
        ),
        (
            "",
            "typescript",
            "fields.ts",
            "comment.posterId",
            picker(
                "posterId: by name, 2 declarations",
                &[
                    ("Issue.posterId", "fields.ts:9"),
                    ("Comment.posterId", "fields.ts:47"),
                ],
            ),
        ),
        (
            "",
            "typescript",
            "fields.ts",
            "comment.body",
            jump(
                "body \u{2192} Comment.body (by name, 1 match)",
                "fields.ts:48",
            ),
        ),
        (
            "`let total` is a local, and `total: 0` the key of an object literal",
            "typescript",
            "fields.ts",
            "comment.total",
            jump("no definition for total", "fields.ts:62"),
        ),
        (
            "",
            "typescript",
            "fields.ts",
            "^  posterId",
            namesakes("posterId", ("Comment.posterId", "fields.ts:47")),
        ),
        (
            "",
            "go",
            "fields.go",
            "issue.PosterID",
            jump(
                "PosterID \u{2192} Issue.PosterID (via issue: Issue)",
                "fields.go:14",
            ),
        ),
        (
            "The second name of `Title, Body string`",
            "go",
            "fields.go",
            "issue.Body",
            jump(
                "Body \u{2192} Issue.Body (via issue: Issue)",
                "fields.go:15",
            ),
        ),
        (
            "Promoted from the embedded `Base`, and the embedded struct by its name",
            "go",
            "fields.go",
            "issue.Audit",
            jump(
                "Audit \u{2192} Base.Audit (via issue: Issue)",
                "fields.go:9",
            ),
        ),
        (
            "Promoted from the embedded `Base`, and the embedded struct by its name",
            "go",
            "fields.go",
            "issue.Base",
            jump(
                "Base \u{2192} Issue.Base (via issue: Issue)",
                "fields.go:13",
            ),
        ),
        (
            "",
            "go",
            "fields.go",
            "i.repo",
            jump("repo \u{2192} Issue.repo (via i: Issue)", "fields.go:16"),
        ),
        (
            "",
            "go",
            "chains.go",
            "h.uow",
            jump("uow \u{2192} Deps.uow (via h: Handler)", "chains.go:25"),
        ),
        (
            "",
            "go",
            "chains.go",
            "h.uow.Users",
            jump(
                "Users \u{2192} UnitOfWork.Users (via h.uow: UnitOfWork)",
                "chains.go:4",
            ),
        ),
        (
            "The variable of a `range` over a channel, which the rules do not read",
            "go",
            "fields.go",
            "c.PosterID",
            picker(
                "PosterID: by name, 2 declarations",
                &[
                    ("Issue.PosterID", "fields.go:14"),
                    ("Comment.PosterID", "fields.go:20"),
                ],
            ),
        ),
        (
            "The variable of a `range` over a channel, which the rules do not read",
            "go",
            "fields.go",
            "c.Text",
            jump(
                "Text \u{2192} Comment.Text (by name, 1 match)",
                "fields.go:21",
            ),
        ),
        (
            "`Host string` in a `var` block is a local",
            "go",
            "fields.go",
            "r.Host",
            jump("no definition for Host", "fields.go:40"),
        ),
        (
            "A field named like its type: on the type, `d` goes to the type",
            "go",
            "fields.go",
            "^\tIssue    *Issue",
            jump("Issue: by name, 1 match", "fields.go:12"),
        ),
        (
            "An embedded struct's name is its type's: `d` there goes to the type",
            "go",
            "fields.go",
            "^\tBase",
            jump("Base: by name, 1 match", "fields.go:8"),
        ),
        (
            "",
            "go",
            "fields.go",
            "^\tPosterID",
            namesakes("PosterID", ("Comment.PosterID", "fields.go:20")),
        ),
    ];
    for (name, fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{name}: {fixture}: {file}: {code}");
    }
}

#[test]
fn super_starts_one_level_up() {
    let cases: Vec<(&str, &str, &str, &str, Shown)> = vec![
        (
            "One base: the nearest declaration above the class, a method or a declared field",
            "python",
            "supers.py",
            "super().__init__",
            jump(
                "__init__ \u{2192} Archive.__init__ (via super of ColdArchive)",
                "supers.py:12",
            ),
        ),
        (
            "One base: the nearest declaration above the class, a method or a declared field",
            "python",
            "supers.py",
            "super().store|(item)",
            jump(
                "store \u{2192} Archive.store (via super of ColdArchive)",
                "supers.py:15",
            ),
        ),
        (
            "An override leads to what it overrides, never to itself; `Generic[T]` declares nothing",
            "python",
            "supers.py",
            "super().store|(item + 1)",
            jump(
                "store \u{2192} ColdArchive.store (via super of GlacierArchive)",
                "supers.py:26",
            ),
        ),
        (
            "An override leads to what it overrides, never to itself; `Generic[T]` declares nothing",
            "python",
            "supers.py",
            "super().flush|()",
            jump(
                "flush \u{2192} Archive.flush (via super of GlacierArchive)",
                "supers.py:18",
            ),
        ),
        (
            "An override leads to what it overrides, never to itself; `Generic[T]` declares nothing",
            "python",
            "supers.py",
            "super().label",
            jump(
                "label \u{2192} Archive.label (via super of GlacierArchive)",
                "supers.py:10",
            ),
        ),
        (
            "A function inside the method: `super()` has no arguments to find there",
            "python",
            "supers.py",
            "super().flush|(), \"no arg",
            picker(
                "flush: by name, 4 declarations",
                &[
                    ("Archive.flush", "supers.py:18"),
                    ("GlacierArchive.flush", "supers.py:36"),
                    ("Right.flush", "supers.py:63"),
                    ("Diamond.flush", "supers.py:68"),
                ],
            ),
        ),
        (
            "Several bases: the first one declaring the member itself is first in any order",
            "python",
            "supers.py",
            "super().store|(item + 2)",
            jump(
                "store \u{2192} Stamped.store (via super of Mixed)",
                "supers.py:44",
            ),
        ),
        (
            "Several bases: the first one declaring the member itself is first in any order",
            "python",
            "supers.py",
            "super().stamp",
            jump(
                "stamp \u{2192} Stamped.stamp (via super of Mixed)",
                "supers.py:47",
            ),
        ),
        (
            "Only one of the bases leads to a `flush`",
            "python",
            "supers.py",
            "super().flush|(), \"one base",
            jump(
                "flush \u{2192} Archive.flush (via super of Mixed)",
                "supers.py:18",
            ),
        ),
        (
            "`Left` leads to `Archive.flush`, `Right` declares its own, and Python asks `Right` first: a walk by depth would jump to the wrong one, so none is proven",
            "python",
            "supers.py",
            "super().flush|(), \"Right",
            picker(
                "flush: by name, 4 declarations",
                &[
                    ("Archive.flush", "supers.py:18"),
                    ("GlacierArchive.flush", "supers.py:36"),
                    ("Right.flush", "supers.py:63"),
                    ("Diamond.flush", "supers.py:68"),
                ],
            ),
        ),
        (
            "`json.JSONEncoder` is outside the project and may declare the member first",
            "python",
            "supers.py",
            "super().store|(item + 3)",
            picker(
                "store: by name, 6 declarations",
                &[
                    ("Archive.store", "supers.py:15"),
                    ("ColdArchive.store", "supers.py:26"),
                    ("GlacierArchive.store", "supers.py:31"),
                    ("Stamped.store", "supers.py:44"),
                    ("Mixed.store", "supers.py:52"),
                    ("Wire.store", "supers.py:73"),
                ],
            ),
        ),
        (
            "`json.JSONEncoder` is outside the project and may declare the member first",
            "python",
            "supers.py",
            "super().default",
            picker(
                "default: by name, 2 declarations",
                &[
                    ("Wire.default", "supers.py:76"),
                    ("Encoder.default", "fields.py:60"),
                ],
            ),
        ),
        (
            "A name between `super()` and the word is not followed: the word is not looked for above the class as if it stood behind `super()` itself",
            "python",
            "supers.py",
            "super().audit.store",
            picker(
                "store: by name, 6 declarations (chain broke at audit)",
                &[
                    ("Archive.store", "supers.py:15"),
                    ("ColdArchive.store", "supers.py:26"),
                    ("GlacierArchive.store", "supers.py:31"),
                    ("Stamped.store", "supers.py:44"),
                    ("Mixed.store", "supers.py:52"),
                    ("Wire.store", "supers.py:73"),
                ],
            ),
        ),
        (
            "TypeScript: one `extends`",
            "typescript",
            "supers.ts",
            "super.store|(item);",
            jump(
                "store \u{2192} Archive.store (via super of ColdArchive)",
                "supers.ts:8",
            ),
        ),
        (
            "TypeScript: one `extends`",
            "typescript",
            "supers.ts",
            "super.store|(item + 1)",
            jump(
                "store \u{2192} ColdArchive.store (via super of GlacierArchive)",
                "supers.ts:18",
            ),
        ),
        (
            "TypeScript: one `extends`",
            "typescript",
            "supers.ts",
            "super.flush",
            jump(
                "flush \u{2192} Archive.flush (via super of GlacierArchive)",
                "supers.ts:10",
            ),
        ),
        (
            "An arrow function passes `super` through as it does `this`",
            "typescript",
            "supers.ts",
            "super.store|(n)",
            jump(
                "store \u{2192} ColdArchive.store (via super of GlacierArchive)",
                "supers.ts:18",
            ),
        ),
        (
            "In an object literal `super` is the literal's prototype, not the class around it",
            "typescript",
            "supers.ts",
            "super.toString",
            picker(
                "toString: by name, 2 declarations",
                &[
                    ("Archive.toString", "supers.ts:12"),
                    ("toString", "supers.ts:32"),
                ],
            ),
        ),
        (
            "The same conditions hold at every level the answer is found through: a diamond or an outside base under the one direct base proves nothing either",
            "python",
            "supers.py",
            "super().drain|(), \"a diamond",
            picker(
                "drain: by name, 6 declarations",
                &[
                    ("Tank.drain", "supers.py:89"),
                    ("LoudTank.drain", "supers.py:102"),
                    ("LeafTank.drain", "supers.py:111"),
                    ("LeafWire.drain", "supers.py:120"),
                    ("SameTank.drain", "supers.py:125"),
                    ("AbcTank.drain", "supers.py:130"),
                ],
            ),
        ),
        (
            "The same conditions hold at every level the answer is found through: a diamond or an outside base under the one direct base proves nothing either",
            "python",
            "supers.py",
            "super().drain|(), \"an outside",
            picker(
                "drain: by name, 6 declarations",
                &[
                    ("Tank.drain", "supers.py:89"),
                    ("LoudTank.drain", "supers.py:102"),
                    ("LeafTank.drain", "supers.py:111"),
                    ("LeafWire.drain", "supers.py:120"),
                    ("SameTank.drain", "supers.py:125"),
                    ("AbcTank.drain", "supers.py:130"),
                ],
            ),
        ),
        (
            "Two bases that lead to the same declaration, and `ABC`, which declares nothing",
            "python",
            "supers.py",
            "super().drain|(), \"both",
            jump(
                "drain \u{2192} Tank.drain (via super of SameTank)",
                "supers.py:89",
            ),
        ),
        (
            "Two bases that lead to the same declaration, and `ABC`, which declares nothing",
            "python",
            "supers.py",
            "super().drain|(), \"ABC",
            jump(
                "drain \u{2192} Tank.drain (via super of AbcTank)",
                "supers.py:89",
            ),
        ),
        (
            "`d` on the word itself is what it was: `super` is no local",
            "python",
            "supers.py",
            "super|().stamp",
            jump("no definition for super", "supers.py:54"),
        ),
        (
            "`d` on the word itself is what it was: `super` is no local",
            "typescript",
            "supers.ts",
            "super|.flush",
            jump("no definition for super", "supers.ts:26"),
        ),
        (
            "`super.open()` returns what the base's `open` declares, not the override's narrower type: a call on `super` is not read as a call on `this`",
            "typescript",
            "supers.ts",
            "opened.store",
            picker(
                "store: by name, 3 declarations",
                &[
                    ("Archive.store", "supers.ts:8"),
                    ("ColdArchive.store", "supers.ts:18"),
                    ("GlacierArchive.store", "supers.ts:24"),
                ],
            ),
        ),
        (
            "`super.open()` returns what the base's `open` declares, not the override's narrower type: a call on `super` is not read as a call on `this`",
            "typescript",
            "supers.ts",
            "super.open().store",
            picker(
                "store: by name, 3 declarations",
                &[
                    ("Archive.store", "supers.ts:8"),
                    ("ColdArchive.store", "supers.ts:18"),
                    ("GlacierArchive.store", "supers.ts:24"),
                ],
            ),
        ),
    ];
    for (name, fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{name}: {fixture}: {file}: {code}");
    }
}

#[test]
fn a_field_rebound_from_itself_ends() {
    let mut a = fixture_app("typescript");
    d_on(&mut a, "fields.ts", "this.close.bind");
    assert_eq!(
        shown(&mut a),
        jump(
            "no definition for bind (chain broke at close)",
            "fields.ts:18"
        ),
        "a field assigned from a call on itself ends the chain"
    );
    let mut a = fixture_app("python");
    d_on(&mut a, "trainer.py", "self.model.forward");
    assert_eq!(
        shown(&mut a),
        jump(
            "forward \u{2192} Model.forward (via self.model: Model)",
            "trainer.py:5",
        ),
        "the field's other assignments still prove its type",
    );
}
