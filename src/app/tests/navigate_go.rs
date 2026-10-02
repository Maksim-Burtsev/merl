use super::*;

/// The rows of the Go section of #100, each on the `go` fixture.
fn go_rows(cases: Vec<(&str, &str, Shown)>) {
    for (file, code, want) in cases {
        let mut a = fixture_app("go");
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{file}: {code}");
    }
}

const BOTH_DELETE_USER: [(&str, &str); 2] = [
    ("UserRepository.DeleteUser", "repos.go:15"),
    ("AuditLog.DeleteUser", "repos.go:21"),
];

/// #100. A Go name no scope of the file declares is the package's: a `var` of another file,
/// of a `var (` block, or below the cursor. A local of the name hides it, readable or not.
#[test]
fn a_go_package_level_name_is_read_in_every_file_of_the_package() {
    let repo = |via: &str| {
        jump(
            &format!("DeleteUser \u{2192} UserRepository.DeleteUser (via {via})"),
            "repos.go:15",
        )
    };
    let audit = |via: &str| {
        jump(
            &format!("DeleteUser \u{2192} AuditLog.DeleteUser (via {via})"),
            "repos.go:21",
        )
    };
    go_rows(vec![
        (
            "globals.go",
            "defaultRepo.DeleteUser|(id + 10",
            repo("defaultRepo: UserRepository"),
        ),
        // The `var` inside `globalsInner` and the raw string's line are not the package's.
        (
            "globals.go",
            "sharedAudit.DeleteUser|(id + 11",
            audit("sharedAudit: AuditLog"),
        ),
        (
            "globals.go",
            "sharedRepo.DeleteUser",
            repo("NewRepo() *UserRepository"),
        ),
        (
            "globals.go",
            "lateRepo.DeleteUser",
            repo("lateRepo: UserRepository"),
        ),
        (
            "globals.go",
            "spareAudit.DeleteUser",
            picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
        ),
        // Declared twice under build tags, as two types.
        (
            "globals.go",
            "taggedRepo.DeleteUser",
            picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
        ),
        // Two files that agree, and two of which one cannot be read.
        (
            "globals.go",
            "twinRepo.DeleteUser",
            repo("twinRepo: UserRepository"),
        ),
        (
            "globals.go",
            "mixedRepo.DeleteUser",
            picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
        ),
        (
            "globals.go",
            "defaultRepo.DeleteUser|(id + 15",
            audit("defaultRepo: AuditLog"),
        ),
        (
            "globals.go",
            "sharedAudit.DeleteUser|(id + 16",
            picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
        ),
        // Locals the scope walk does not read: nothing is proven from their absence.
        (
            "globals.go",
            "defaultRepo.DeleteUser|(18",
            picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
        ),
        (
            "globals.go",
            "defaultRepo.DeleteUser|(19",
            picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
        ),
        (
            "globals.go",
            "defaultRepo.DeleteUser|(20",
            picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
        ),
        // A label opens no block: the local above it is read (#330).
        (
            "globals.go",
            "defaultRepo.DeleteUser|(21",
            audit("defaultRepo: AuditLog"),
        ),
        (
            "globals.go",
            "hop.DeleteUser",
            picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
        ),
        // An import of the external test package is no variable of `package main`.
        (
            "globals_x_test.go",
            "session.Close",
            jump(
                "Close \u{2192} Session.Close (via defaultRepo.Open() *Session)",
                "store/store.go:15",
            ),
        ),
    ]);
}

/// #100. Go's `type X = Y` is followed to `Y`, through a second alias and into another
/// package; `type X Y` declares a type with methods of its own.
#[test]
fn a_go_alias_is_the_type_it_names() {
    go_rows(vec![
        (
            "aliases.go",
            "first.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via first: UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "aliases.go",
            "again.DeleteUser",
            jump(
                "DeleteUser \u{2192} UserRepository.DeleteUser (via again: UserRepository)",
                "repos.go:15",
            ),
        ),
        (
            "aliases.go",
            "session.Close",
            jump(
                "Close \u{2192} Session.Close (via session: Session)",
                "store/store.go:15",
            ),
        ),
        (
            "aliases.go",
            "twin.Close",
            jump(
                "Close \u{2192} Session.Close (via twin: Session)",
                "store/store.go:15",
            ),
        ),
        (
            "aliases.go",
            "kind.Flush",
            jump(
                "Flush \u{2192} AuditKind.Flush (via kind: AuditKind)",
                "aliases.go:19",
            ),
        ),
    ]);
    let mut a = fixture_app("go");
    d_on(&mut a, "aliases.go", "twin.Seal");
    assert_eq!(
        shown(&mut a),
        jump(
            "Seal \u{2192} AuditTwin.Seal (via twin: AuditTwin)",
            "aliases.go:39"
        )
    );
    let (dir, mut a) = project_app(
        "alias-cycle",
        &[(
            "a.go",
            "package a\n\ntype A = B\n\ntype B = A\n\ntype C struct{}\n\nfunc (c C) Run() {}\n\nfunc f(x A) {\n\tx.Run()\n}\n",
        )],
    );
    a.external
        .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "a.go", "x.Run");
    assert_eq!(
        a.message, "Run \u{2192} C.Run (by name, 1 match)",
        "Two aliases of each other, which no compiler accepts, end"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #100. A Go type declared once per platform (`clock_windows.go` beside a
/// `//go:build !windows` file) is the one the host builds, and one declared under a tag of
/// the project's own is the one a plain `go build` compiles, or the one `-tags` asks for
/// (#137). From inside a file that is not built nothing is preferred.
#[test]
fn a_go_declaration_per_platform_is_the_hosts() {
    let (mine, other) = match cfg!(windows) {
        true => ("clock_windows.go", "clock_other.go"),
        false => ("clock_other.go", "clock_windows.go"),
    };
    let host = if cfg!(windows) {
        ("windows", 3)
    } else {
        ("other", 5)
    };
    let gauge = if cfg!(windows) {
        ("fast", 8)
    } else {
        ("other", 7)
    };
    let now = |file: &str| format!("platform/{file}:11");
    let via = |via: &str| format!("Now \u{2192} Clock.Now (via {via})");
    let both = |status: &str, name: &str, a: &str, b: &str| {
        Shown::Picker(
            status.into(),
            vec![
                (name.into(), "by name".into(), a.into()),
                (name.into(), "by name".into(), b.into()),
            ],
        )
    };
    go_rows(vec![
        (
            "platforms.go",
            "clock.Now",
            jump(&via("clock: Clock"), &now(mine)),
        ),
        (
            "platforms.go",
            "made.Now",
            jump(&via("platform.NewClock() *Clock"), &now(mine)),
        ),
        (
            "platforms.go",
            "platform.NewClock",
            jump(
                "NewClock: via import platform/",
                &format!("platform/{mine}:7"),
            ),
        ),
        (
            "platforms.go",
            "codec.Encode",
            jump(
                "Encode \u{2192} Codec.Encode (via codec: Codec)",
                "platform/codec_slow.go:7",
            ),
        ),
        (
            "platform/codec_fast.go",
            "c.Encode",
            both(
                "Encode: by name, 2 declarations",
                "Codec.Encode",
                "platform/codec_fast.go:8",
                "platform/codec_slow.go:7",
            ),
        ),
        // The type is declared once and its method per platform.
        (
            "platforms.go",
            "timer.Tick",
            jump(
                "Tick \u{2192} Timer.Tick (via timer: Timer)",
                &format!("platform/timer_{}.go:{}", host.0, host.1),
            ),
        ),
        // A platform and a tag of the project's own: `windows && !slow`.
        (
            "platforms.go",
            "gauge.Read",
            jump(
                "Read \u{2192} Gauge.Read (via gauge: Gauge)",
                &format!("platform/gauge_{}.go:{}", gauge.0, gauge.1),
            ),
        ),
        (
            &format!("platform/{other}"),
            "c.Now",
            both(
                "Now: by name, 2 declarations",
                "Clock.Now",
                &now(other),
                &now(mine),
            ),
        ),
    ]);
    let mut a = fixture_app("go");
    a.go_build.tags = vec!["fast".into()];
    d_on(&mut a, "platforms.go", "codec.Encode");
    assert_eq!(
        shown(&mut a),
        jump(
            "Encode \u{2192} Codec.Encode (via codec: Codec)",
            "platform/codec_fast.go:8",
        ),
        "`GOFLAGS=-tags=fast` builds the other one"
    );
    // Asked from inside the file the host does not build, a method per platform is both;
    // and where no declaration is built (`gate_windows.go`, `gate_plan9.go`), all stay.
    if !cfg!(windows) {
        let mut a = fixture_app("go");
        d_on(&mut a, "platform/timer_windows.go", "t.Tick");
        let row = |place: &str| ("Timer.Tick".into(), "via t: Timer".into(), place.into());
        assert_eq!(
            shown(&mut a),
            Shown::Picker(
                "Tick: via t: Timer, 2 declarations".into(),
                vec![
                    row("platform/timer_windows.go:3"),
                    row("platform/timer_other.go:5")
                ],
            )
        );
        d_on(&mut a, "platforms_gate.go", "gate.Lift");
        assert_eq!(
            shown(&mut a),
            both(
                "Lift: by name, 2 declarations",
                "Gate.Lift",
                "platform/gate_plan9.go:5",
                "platform/gate_windows.go:6",
            )
        );
        d_on(&mut a, "platforms_gate.go", "platform.NewGate");
        assert_eq!(a.message, "NewGate: via import platform/, 2 declarations");
        press(&mut a, KeyCode::Esc, KeyModifiers::NONE);
        d_on(&mut a, "platforms_gate.go", "meter.Sample");
        assert_eq!(
            shown(&mut a),
            both(
                "Sample: by name, 2 declarations",
                "Meter.Sample",
                "platform/meter_fast.go:8",
                "platform/meter_windows.go:5",
            ),
            "`meter_fast.go` (`// +build`) is not known to be built, so `meter_windows.go` loses to nothing"
        );
    }
}

/// #100. `d` on a Go package qualifier is the import line of the open file. A local of the
/// name is the local, and a name the package declares itself is not the import its path
/// happens to spell.
#[test]
fn a_go_package_qualifier_is_its_import_line() {
    go_rows(vec![
        (
            "qualifiers.go",
            "depot|.Open",
            jump(
                "depot: via import example.com/fixture/store",
                "qualifiers.go:7",
            ),
        ),
        (
            "qualifiers.go",
            "fmt|.Println",
            jump("fmt: via import fmt", "qualifiers.go:4"),
        ),
        // On the import line itself the name is no qualifier.
        (
            "qualifiers.go",
            "depot| \"example",
            jump("no definition for depot", "qualifiers.go:7"),
        ),
        (
            "qualifiers.go",
            "depot|.Remove",
            jump(
                "depot \u{2192} QualifierHidden.depot (local)",
                "qualifiers.go:19",
            ),
        ),
        (
            "qualifiers.go",
            "depot|.Remove(2",
            jump("no definition for depot", "qualifiers.go:32"),
        ),
        (
            "qualifiers.go",
            "h.depot|.Remove",
            jump(
                "depot \u{2192} qualifierHolder.depot (via h: qualifierHolder)",
                "qualifiers.go:37",
            ),
        ),
        (
            "qualifiers.go",
            "ledger|.DeleteUser",
            picker(
                "ledger: by name, 5 declarations",
                &[
                    ("ledger", "scopes.go:3"),
                    ("ScopedRotate.ledger", "scopes.go:10"),
                    ("ScopedNested.ledger", "scopes.go:15"),
                    ("ledger", "scopes.go:17"),
                    ("ledger", "scopes.go:34"),
                ],
            ),
        ),
    ]);
}

/// #100. A named result or a parameter of a Go function is named after the function, as a
/// local of its body is: `Reload.err`, not `UserRepository.err`, which would be a field.
#[test]
fn a_go_named_result_is_named_after_its_function() {
    go_rows(vec![
        (
            "results.go",
            "return user, err",
            jump("err \u{2192} Reload.err (local)", "results.go:4"),
        ),
        (
            "results.go",
            "count| == 0",
            jump("count \u{2192} Reload.count (local)", "results.go:5"),
        ),
        (
            "results.go",
            "FindUser(id|)",
            jump("id \u{2192} Reload.id (local)", "results.go:4"),
        ),
        (
            "results.go",
            "\treturn err",
            jump("err \u{2192} ReloadPlain.err (local)", "results.go:12"),
        ),
    ]);
}

/// Step 6 of #68 over the same project in three languages: on the declaration of a member of
/// an interface, a protocol, an abstract or a base class, `d` offers what implements it,
/// labelled with the member it comes from. A type that inherits the member without declaring
/// it, a member of another number of parameters and a declaration nothing implements are left
/// out, and the last of those falls back to the search by name.
#[test]
fn implementations_are_offered_on_the_declaration_they_implement() {
    let impls = |status: &str, member: &str, rows: &[(&str, &str)]| {
        let why = format!("implementations of {member}");
        let rows = rows
            .iter()
            .map(|(name, place)| (name.to_string(), why.clone(), place.to_string()))
            .collect();
        Shown::Picker(status.into(), rows)
    };
    let cases: [(&str, &str, &str, Shown); 9] = [
        // A base class: the subclasses that override it, `NightlyJob` two levels down.
        // `QuietJob` inherits `run` without declaring it and is no implementation.
        (
            "python",
            "impls.py",
            "def run",
            impls(
                "run: implementations of BaseJob.run, 5 declarations",
                "BaseJob.run",
                &[
                    ("ImportJob.run", "impls.py:10"),
                    ("ExportJob.run", "impls.py:15"),
                    // A header black wrapped, one base to a line (#100), and a class
                    // below it. `Roster` has a `BaseJob,` line too, an argument of a call.
                    ("WrappedJob.run", "impls.py:57"),
                    ("NightlyJob.run", "impls.py:24"),
                    ("DeepJob.run", "impls.py:81"),
                ],
            ),
        ),
        // A protocol is structural: `WebhookNotifier` names nothing and implements it,
        // `Batch.send` takes another parameter and does not.
        (
            "python",
            "repos.py",
            "def send",
            impls(
                "send: implementations of Notifier.send, 4 declarations",
                "Notifier.send",
                &[
                    ("EmailNotifier.send", "repos.py:22"),
                    ("SmsNotifier.send", "repos.py:27"),
                    ("LoudNotifier.send", "impls.py:29"),
                    ("WebhookNotifier.send", "impls.py:34"),
                ],
            ),
        ),
        (
            "typescript",
            "impls.ts",
            "^  run",
            impls(
                "run: implementations of BaseJob.run, 4 declarations",
                "BaseJob.run",
                &[
                    ("ImportJob.run", "impls.ts:8"),
                    ("ExportJob.run", "impls.ts:12"),
                    // A header prettier wrapped over three lines is read all the same.
                    ("WrappedJob.run", "impls.ts:39"),
                    ("NightlyJob.run", "impls.ts:18"),
                ],
            ),
        ),
        // `implements` in the same file and behind an import.
        (
            "typescript",
            "repos.ts",
            "^  send",
            impls(
                "send: implementations of Notifier.send, 4 declarations",
                "Notifier.send",
                &[
                    ("EmailNotifier.send", "repos.ts:26"),
                    ("SmsNotifier.send", "repos.ts:32"),
                    ("LoudNotifier.send", "impls.ts:22"),
                    ("WrappedJob.send", "impls.ts:41"),
                ],
            ),
        ),
        // Go's interfaces are implicit: the method name and the number of parameters are all
        // there is to go on, and `Batch.Run` takes one more.
        (
            "go",
            "impls.go",
            "Run",
            impls(
                "Run: implementations of Job.Run, 2 declarations",
                "Job.Run",
                &[
                    ("ImportJob.Run", "impls.go:11"),
                    ("ExportJob.Run", "impls.go:17"),
                ],
            ),
        ),
        (
            "go",
            "repos.go",
            "Send",
            impls(
                "Send: implementations of Notifier.Send, 3 declarations",
                "Notifier.Send",
                &[
                    ("EmailNotifier.Send", "repos.go:31"),
                    ("SmsNotifier.Send", "repos.go:37"),
                    ("LoudNotifier.Send", "impls.go:29"),
                ],
            ),
        ),
        // One implementation is an answer, not a one-row picker, and the status line says
        // where it came from.
        (
            "typescript",
            "impls.ts",
            "^  sweep",
            jump(
                "sweep \u{2192} NightlySweeper.sweep (implementations of Sweeper.sweep)",
                "impls.ts:32",
            ),
        ),
        // Nothing implements a Go method beside its type: the search by name answers, and
        // says the cursor is on one of them.
        (
            "go",
            "repos.go",
            "func (e *EmailNotifier) Send",
            picker(
                "Send: at a declaration, 2 others by name",
                &[
                    ("SmsNotifier.Send", "repos.go:37"),
                    ("LoudNotifier.Send", "impls.go:29"),
                ],
            ),
        ),
        // A member of a value still resolves through the type of the receiver, not through
        // the implementations of the interface it lands on.
        (
            "python",
            "service.py",
            "self.notifier.send",
            jump(
                "send \u{2192} Notifier.send (via self.notifier: Notifier)",
                "repos.py:18",
            ),
        ),
    ];
    for (fixture, file, code, want) in cases {
        let mut a = fixture_app(fixture);
        d_on(&mut a, file, code);
        assert_eq!(shown(&mut a), want, "{fixture}: {file}: {code}");
    }
}

/// #517. Implementations answer only on the name a member's line declares: another occurrence
/// of the word on that line, a namesake its one-line body calls, a parameter or a type its
/// signature names, is looked up as on any other line.
#[test]
fn implementations_answer_only_on_the_declared_name() {
    let (dir, mut a) = project_app(
        "d-517-impls",
        &[
            (
                "jobs.py",
                "class BaseJob:\n    def run(self): return run()\n\n\nclass ImportJob(BaseJob):\n    def run(self): pass\n\n\ndef run(): pass\n",
            ),
            (
                "jobs.ts",
                "export class BaseJob {\n  run(run: () => void) {\n    run();\n  }\n}\n\nexport class ImportJob extends BaseJob {\n  run(run: () => void) {}\n}\n",
            ),
            ("go.mod", "module example.com/jobs\n"),
            (
                "jobs.go",
                "package jobs\n\ntype Send string\n\ntype Notifier interface {\n\tSend(msg Send) error\n\tWave(w Wave) error\n}\n\ntype Email struct{}\n\nfunc (e Email) Send(msg Send) error { return nil }\n\nfunc (e Email) Wave(w Wave) error { return nil }\n",
            ),
        ],
    );
    // Off the declared name, the search by name answers, as on a line nothing implements.
    let cases = [
        (
            "jobs.py",
            "def run",
            "return run",
            picker(
                "run: at a declaration, 2 others by name",
                &[("ImportJob.run", "jobs.py:6"), ("run", "jobs.py:9")],
            ),
        ),
        (
            "jobs.ts",
            "^  run",
            "(run",
            picker(
                "run: at a declaration, 1 other by name",
                &[("ImportJob.run", "jobs.ts:8")],
            ),
        ),
        (
            "jobs.go",
            "\tSend",
            "(msg Send",
            // A parameter's type is looked up as a type (#536).
            jump("Send: by name, 1 match", "jobs.go:3"),
        ),
        (
            "jobs.go",
            "\tWave",
            "(w Wave",
            // With no type of that name, the namesakes are offered, as before #536.
            picker(
                "Wave: at a declaration, 1 other by name",
                &[("Email.Wave", "jobs.go:14")],
            ),
        ),
    ];
    for (file, declared, other, want) in cases {
        d_on(&mut a, file, declared);
        assert!(
            a.message.contains("implementations of"),
            "{file}: {declared}: {}",
            a.message
        );
        d_on(&mut a, file, other);
        assert_eq!(shown(&mut a), want, "{file}: {other}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

/// The two presses #68 step 6 is reached by: `d` on a call of an interface method lands on
/// the declaration with the cursor on its name, and a second `d` there lists what implements
/// it. A jump that left the cursor at the start of the line would answer nothing.
#[test]
fn a_second_d_on_the_declaration_a_jump_landed_on_lists_the_implementations() {
    let mut a = fixture_app("python");
    d_on(&mut a, "service.py", "self.notifier.send");
    assert_eq!(
        a.message,
        "send \u{2192} Notifier.send (via self.notifier: Notifier)"
    );
    assert_eq!(format!("{}:{}", a.rel_path(), a.line + 1), "repos.py:18");
    assert_eq!(&a.line_str()[a.col..a.col + 4], "send", "on the word");
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    assert_eq!(
        a.message,
        "send: implementations of Notifier.send, 4 declarations"
    );
}

/// #325. A Go raw string has no escapes, so a backslash before its closing backtick is a
/// backslash: the declarations after `` `\` ``, `` `C:\` `` and `` `{\\\}\\\\` `` are found.
#[test]
fn a_go_raw_string_ending_in_a_backslash_ends_there() {
    let (dir, mut a) = project_app(
        "go-raw-backslash",
        &[
            ("go.mod", "module example.com/rawstr\n"),
            (
                "shop/shop.go",
                "package shop\n\nimport \"strings\"\n\nfunc slash(p string) string { return below(strings.ReplaceAll(p, `\\`, \"/\")) }\n\nfunc below(p string) string { return p }\n",
            ),
            (
                "shop/drive.go",
                "package shop\n\nfunc root() string { return drive() + volume() }\n\nfunc drive() string { return `C:\\` }\n\nfunc volume() string { return \"\" }\n",
            ),
            (
                "shop/braces.go",
                "package shop\n\nfunc pattern() string { return braces() + tail() }\n\nfunc braces() string { return `{\\\\\\}\\\\\\\\` }\n\nfunc tail() string { return \"\" }\n",
            ),
        ],
    );
    a.external
        .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
    let mut d = |file: &str, code: &str| {
        d_on(&mut a, file, code);
        shown(&mut a)
    };
    let by_name = |word: &str, place: &str| jump(&format!("{word}: by name, 1 match"), place);
    assert_eq!(
        d("shop/shop.go", "return below"),
        by_name("below", "shop/shop.go:7")
    );
    assert_eq!(
        d("shop/shop.go", "func slash"),
        by_name("slash", "shop/shop.go:5"),
        "On its declaration `d` says what it says on any other, as on `slash` above the string"
    );
    assert_eq!(
        d("shop/shop.go", "func below"),
        by_name("below", "shop/shop.go:7")
    );
    assert_eq!(
        d("shop/drive.go", "+ volume"),
        by_name("volume", "shop/drive.go:7")
    );
    assert_eq!(
        d("shop/braces.go", "+ tail"),
        by_name("tail", "shop/braces.go:7")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #476. Go's blank identifier names nothing: `d` on `_` says so without a search, and jumps to
/// no earlier `_`. In Python `_` is a name like any other and is still found.
#[test]
fn the_go_blank_identifier_has_no_definition() {
    let (dir, mut a) = project_app(
        "go-blank",
        &[
            ("go.mod", "module example.com/blank\n"),
            (
                "main.go",
                "package main\n\nfunc pair() (int, int) { return 1, 2 }\n\nfunc main() {\n\t_, a := pair()\n\t_, b := pair()\n\tprintln(a, b)\n}\n",
            ),
            ("tr.py", "_ = str\n\nprint(_(1))\n"),
        ],
    );
    a.external
        .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "main.go", "\t_|, b");
    assert_eq!(shown(&mut a), jump("no definition for _", "main.go:7"));
    d_on(&mut a, "tr.py", "print(_");
    assert_eq!(shown(&mut a), jump("_: local", "tr.py:1"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #326. A member of a column-0 `const (`, `var (` or `type (` block is a top-level declaration,
/// in the project and in a package outside it; a field of a struct inside `type (` and a `var (`
/// block inside a function are not.
#[test]
fn a_go_grouped_declaration_is_top_level() {
    let (dir, mut a) = project_app(
        "go-grouped",
        &[
            ("go.mod", "module example.com/grouped\n"),
            (
                "shop/shop.go",
                "package shop\n\nimport \"time\"\n\nconst (\n\tsortByName = \"name\"\n\tKindA      = iota\n\tKindB\n)\n\nvar (\n\tDefaultTimeout = 5 * time.Second\n)\n\ntype (\n\tOrder struct {\n\t\tAddress string\n\t}\n)\n\nfunc sorted(s string) bool { return s == sortByName }\n\nfunc wait() time.Duration { return DefaultTimeout + time.Hour }\n\nfunc build() Order { return Order{} }\n\nfunc kind() int { return KindB }\n\nfunc local() string {\n\tvar (\n\t\tStreet = \"x\"\n\t)\n\treturn Street\n}\n\nfunc Address() string { return \"\" }\n\nfunc street() string { return Street + Address() }\n",
            ),
        ],
    );
    let goroot = external_root(
        "go-grouped",
        &[(
            "src/time/time.go",
            "package time\n\ntype Duration int64\n\nconst (\n\tNanosecond Duration = 1\n\tMinute             = 60 * Nanosecond\n\tHour               = 60 * Minute\n)\n",
        )],
    );
    use_roots(&mut a, Kind::Go, &[goroot.join("src")]);
    let mut d = |code: &str| {
        d_on(&mut a, "shop/shop.go", code);
        shown(&mut a)
    };
    let by_name = |word: &str, place: &str| jump(&format!("{word}: by name, 1 match"), place);
    assert_eq!(
        d("s == sortByName"),
        by_name("sortByName", "shop/shop.go:6")
    );
    assert_eq!(
        d("return DefaultTimeout"),
        by_name("DefaultTimeout", "shop/shop.go:12")
    );
    let hour = goroot.join("src/time/time.go");
    assert_eq!(
        d("time.Hour"),
        jump("Hour: via import time", &format!("{}:8", hour.display()))
    );
    assert_eq!(d("return Order"), by_name("Order", "shop/shop.go:16"));
    assert_eq!(d("return KindB"), by_name("KindB", "shop/shop.go:8"));
    assert_eq!(
        d("+ Address"),
        by_name("Address", "shop/shop.go:36"),
        "The field of `Order` is no top-level `Address`, and the function's own `var (` block declares a local, not a name of the package"
    );
    assert_eq!(
        d("return Street| +"),
        jump("no definition for Street", "shop/shop.go:38")
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&goroot).unwrap();
}

/// #327. A key of a composite literal is a field of the literal's type: written in front of the
/// `{`, or the element type of the literal around an elided `{`. A map's keys are values, and so
/// are a slice expression's, a label and a `case`; a literal whose type is outside the project
/// offers what the name finds.
#[test]
fn a_go_literal_key_is_a_field_of_the_literals_type() {
    let (dir, mut a) = project_app(
        "go-keys",
        &[
            ("go.mod", "module example.com/keys\n"),
            (
                "shop/shop.go",
                "package shop\n\ntype Order struct {\n\tAddress string\n\tItems   []Item\n}\n\ntype Item struct{ Name string }\n\ntype Address struct{ Street string }\n\nfunc (a Address) Name() string { return a.Street }\n\nfunc build(addr string) Order {\n\treturn Order{\n\t\tAddress: addr,\n\t\tItems: []Item{\n\t\t\t{Name: \"a\"},\n\t\t},\n\t}\n}\n",
            ),
            (
                "shop/more.go",
                "package shop\n\nconst Street = \"s\"\n\nvar names = map[string]string{Street: \"a\"}\n\nvar nested = map[string][]Item{\"k\": {{Name: \"b\"}}}\n\nfunc cut(xs []int, Street int) []int { return xs[Street:] }\n\nfunc label(k string) int {\nStreet:\n\tfor range 3 {\n\t\tbreak Street\n\t}\n\tswitch k {\n\tcase Street:\n\t\treturn 1\n\t}\n\treturn 0\n}\n\nfunc none() Order { return Order{Missing: 1} }\n",
            ),
            (
                "app/app.go",
                "package app\n\nimport (\n\t\"sync\"\n\n\t\"example.com/keys/shop\"\n)\n\nvar pool = sync.Pool{\n\tNew: func() any { return nil },\n}\n\nfunc order() *shop.Order { return &shop.Order{Address: \"x\"} }\n",
            ),
            (
                "other/other.go",
                "package other\n\nfunc New() int { return 1 }\n",
            ),
        ],
    );
    a.external
        .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
    let mut d = |file: &str, code: &str| {
        d_on(&mut a, file, code);
        shown(&mut a)
    };
    assert_eq!(
        d("shop/shop.go", "\t\tAddress|: addr"),
        jump(
            "Address \u{2192} Order.Address (via Order{\u{2026}})",
            "shop/shop.go:4"
        )
    );
    assert_eq!(
        d("shop/shop.go", "{Name"),
        jump(
            "Name \u{2192} Item.Name (via Item{\u{2026}})",
            "shop/shop.go:8"
        )
    );
    assert_eq!(
        d("shop/more.go", "{{Name"),
        jump(
            "Name \u{2192} Item.Name (via Item{\u{2026}})",
            "shop/shop.go:8"
        )
    );
    assert_eq!(
        d("app/app.go", "{Address"),
        jump(
            "Address \u{2192} Order.Address (via shop.Order{\u{2026}})",
            "shop/shop.go:4"
        )
    );
    assert_eq!(
        d("app/app.go", "\tNew"),
        picker("New: by name, 1 match", &[("New", "other/other.go:3")]),
        "`sync.Pool` is outside the project: what the name finds is offered, never jumped to"
    );
    assert_eq!(
        d("shop/more.go", "Order{Missing"),
        jump("no definition for Missing", "shop/more.go:23"),
        "A struct without the field says so"
    );
    let street = || jump("Street: local", "shop/more.go:3");
    assert_eq!(
        d("shop/more.go", "{Street"),
        street(),
        "A map's keys, a slice expression, a label and a `case` are no fields. In the slice expression it is the parameter of the function on its line (#524)"
    );
    assert_eq!(
        d("shop/more.go", "xs[Street"),
        jump("Street \u{2192} cut.Street (local)", "shop/more.go:9")
    );
    assert_eq!(d("shop/more.go", "case Street"), street());
    assert_eq!(d("shop/more.go", "^Street"), street());
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #332. A bare Go name is a local, a top-level name of the file's own package, a name of a dot
/// import or a predeclared one, and `pkg.X` is declared in `pkg`'s directory or nowhere: no
/// namesake of another package is offered.
#[test]
fn a_go_name_is_looked_up_in_its_own_package() {
    let (dir, mut a) = project_app(
        "go-confine",
        &[
            ("go.mod", "module example.com/confine\n"),
            (
                "shop/shop.go",
                "package shop\n\nconst (\n\tMaxItems = 10\n)\n\ntype NetworkAddress struct{ Network string }\n\nfunc (na NetworkAddress) IsUnix() bool { return false }\n\nfunc IsUnix(n string) bool { return n == \"unix\" }\n\nfunc check(n string) bool { return IsUnix(n) }\n\nfunc Network() string { return \"\" }\n\nvar addr = NetworkAddress{Network: \"tcp\"}\n",
            ),
            (
                "shop/inner_test.go",
                "package shop\n\nfunc inner() bool { return IsUnix(\"x\") && extOnly() }\n",
            ),
            (
                "shop/outer_test.go",
                "package shop_test\n\nfunc extOnly() bool { return IsUnix(\"x\") }\n",
            ),
            (
                "other/other.go",
                "package other\n\nfunc MaxItems() int { return 3 }\n\nfunc helper() {\n\tcount := 1\n\t_ = count\n}\n",
            ),
            (
                "app/app.go",
                "package app\n\nimport \"example.com/confine/shop\"\n\nvar (\n\tcount         = 0\n\tvalidUserinfo = true\n)\n\nfunc limit() int { return shop.MaxItems }\n\nfunc total() int { return count }\n\nfunc valid() bool { return validUserinfo }\n\nfunc size(xs []int) int { return len(xs) }\n\nfunc unknown() int { return shop.Missing }\n",
            ),
            (
                "dot/dot.go",
                "package dot\n\nimport . \"example.com/confine/shop\"\n\nfunc most() int { return MaxItems }\n",
            ),
        ],
    );
    let goroot = external_root(
        "go-confine",
        &[
            (
                "src/builtin/builtin.go",
                "package builtin\n\ntype Type int\n\nfunc len(v Type) int\n",
            ),
            (
                "src/net/url/url.go",
                "package url\n\nfunc validUserinfo(s string) bool { return true }\n",
            ),
        ],
    );
    use_roots(&mut a, Kind::Go, &[goroot.join("src")]);
    let mut d = |file: &str, code: &str| {
        d_on(&mut a, file, code);
        shown(&mut a)
    };
    let by_name = |word: &str, place: &str| jump(&format!("{word}: by name, 1 match"), place);
    assert_eq!(
        d("shop/shop.go", "return IsUnix"),
        by_name("IsUnix", "shop/shop.go:11"),
        "A bare name is never a method"
    );
    assert_eq!(
        d("app/app.go", "shop.MaxItems"),
        jump("MaxItems: via import shop/", "shop/shop.go:4")
    );
    assert_eq!(
        d("app/app.go", "shop.Missing"),
        jump("no definition for Missing", "app/app.go:18")
    );
    assert_eq!(
        d("app/app.go", "return count"),
        by_name("count", "app/app.go:6")
    );
    assert_eq!(
        d("app/app.go", "return validUserinfo"),
        by_name("validUserinfo", "app/app.go:7")
    );
    assert_eq!(
        d("app/app.go", "return len"),
        jump(
            "len: via builtin",
            &format!("{}:5", goroot.join("src/builtin/builtin.go").display())
        )
    );
    assert_eq!(
        d("dot/dot.go", "return MaxItems"),
        jump("MaxItems: via import shop/", "shop/shop.go:4")
    );
    assert_eq!(
        d("shop/inner_test.go", "return IsUnix"),
        by_name("IsUnix", "shop/shop.go:11"),
        "An external test package shares the directory, not the names"
    );
    assert_eq!(
        d("shop/inner_test.go", "&& extOnly"),
        jump("no definition for extOnly", "shop/inner_test.go:3")
    );
    assert_eq!(
        d("shop/outer_test.go", "return IsUnix"),
        jump("no definition for IsUnix", "shop/outer_test.go:3")
    );
    assert_eq!(
        d("shop/shop.go", "{Network"),
        jump(
            "Network \u{2192} NetworkAddress.Network (via NetworkAddress{\u{2026}})",
            "shop/shop.go:7"
        ),
        "A key is the literal's field, whatever the package declares of its name"
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&goroot).unwrap();
}

/// #330. The scope walk reads a table test: the variable of `range []struct {…}{…}` and of a
/// range over `tests := []struct {…}{…}` is the struct written in place, and a key of one of its
/// elements is its field. A label opens no block, and a `:=` that redeclares a name reuses the
/// one its block declared first.
#[test]
fn a_go_table_test_struct_is_a_type() {
    let (dir, mut a) = project_app(
        "go-walk",
        &[
            ("go.mod", "module example.com/walk\n"),
            (
                "shop/shop.go",
                "package shop\n\nimport \"strings\"\n\nfunc labelled(input string) string {\n\tvar sb strings.Builder\n\tsb.Grow(len(input))\nscan:\n\tfor i := 0; i < len(input); i++ {\n\t\tif input[i] == 'x' {\n\t\t\tcontinue scan\n\t\t}\n\t\tsb.WriteString(\"y\")\n\t}\n\treturn sb.String()\n}\n\nfunc table() {\n\tfor _, tc := range []struct {\n\t\tname  string\n\t\tcheck func(error) bool\n\t}{\n\t\t{name: \"a\"},\n\t} {\n\t\t_ = tc.name\n\t\t_ = tc.check(nil)\n\t}\n\ttests := []struct{ want int }{{want: 1}}\n\tfor _, tt := range tests {\n\t\t_ = tt.want\n\t}\n}\n\nfunc redeclared() error {\n\terr := first()\n\tif err != nil {\n\t\treturn err\n\t}\n\tn, err := second()\n\t_ = n\n\terr = first()\n\treturn err\n}\n\nfunc first() error { return nil }\n\nfunc second() (int, error) { return 0, nil }\n",
            ),
            (
                "other/other.go",
                "package other\n\nfunc helper() {\n\tsb := 1\n\t_ = sb\n\tname := \"x\"\n\t_ = name\n}\n",
            ),
        ],
    );
    a.external
        .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
    let mut d = |code: &str| {
        d_on(&mut a, "shop/shop.go", code);
        shown(&mut a)
    };
    let sb = || jump("sb \u{2192} labelled.sb (local)", "shop/shop.go:6");
    assert_eq!(d("\t\tsb|.WriteString"), sb());
    assert_eq!(d("return sb|.String"), sb());
    let field = |word: &str, place: &str| {
        jump(
            &format!("{word} \u{2192} struct{{\u{2026}}}.{word} (via tc: struct{{\u{2026}}})"),
            place,
        )
    };
    assert_eq!(d("tc.name"), field("name", "shop/shop.go:20"));
    assert_eq!(d("tc.check"), field("check", "shop/shop.go:21"));
    assert_eq!(
        d("{name"),
        jump(
            "name \u{2192} struct{\u{2026}}.name (via struct{\u{2026}})",
            "shop/shop.go:20"
        )
    );
    assert_eq!(
        d("tt.want"),
        jump(
            "want \u{2192} struct{\u{2026}}.want (via tt: struct{\u{2026}})",
            "shop/shop.go:28"
        )
    );
    assert_eq!(
        d("\terr| = first"),
        jump("err \u{2192} redeclared.err (local)", "shop/shop.go:35")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// #334. A receiver whose type is declared in the standard library or a module is proven there:
/// a struct's field and method, an interface's method line, a method of an embedded type, a
/// type reached through one call, and a literal's key.
#[test]
fn a_go_type_outside_the_project_is_proven() {
    let (dir, mut a) = project_app(
        "go-external",
        &[
            (
                "go.mod",
                "module example.com/external\n\nrequire example.com/lib v1.0.0\n",
            ),
            (
                "app/app.go",
                "package app\n\nimport (\n\t\"context\"\n\t\"net/http\"\n\t\"net/http/httptest\"\n\t\"sync\"\n\t\"testing\"\n\n\t\"example.com/lib\"\n)\n\nfunc handle(ctx context.Context, r *http.Request, t *testing.T) string {\n\tvar wg sync.WaitGroup\n\twg.Add(1)\n\tt.Errorf(\"x\")\n\t_ = ctx.Err()\n\tsrv := httptest.NewServer(nil)\n\tsrv.Close()\n\tvar c lib.Config\n\t_ = c.Name\n\t_ = sync.Pool{New: nil}\n\treturn r.URL.Path\n}\n",
            ),
            (
                "app/other.go",
                "package app\n\ntype other struct{}\n\nfunc (o other) Add(n int) {}\n\nfunc (o other) Close() {}\n",
            ),
        ],
    );
    let root = external_root(
        "go-external",
        &[
            (
                "src/sync/waitgroup.go",
                "package sync\n\ntype WaitGroup struct {\n\tn int\n}\n\nfunc (wg *WaitGroup) Add(delta int) {}\n",
            ),
            (
                "src/sync/pool.go",
                "package sync\n\ntype Pool struct {\n\tNew func() any\n}\n",
            ),
            (
                "src/testing/testing.go",
                "package testing\n\ntype common struct{}\n\nfunc (c *common) Errorf(format string, args ...any) {}\n\ntype T struct {\n\tcommon\n\tname string\n}\n",
            ),
            (
                "src/context/context.go",
                "package context\n\ntype Context interface {\n\tErr() error\n}\n",
            ),
            (
                "src/net/http/request.go",
                "package http\n\nimport \"net/url\"\n\ntype Request struct {\n\tURL *url.URL\n}\n",
            ),
            (
                "src/net/url/url.go",
                "package url\n\ntype URL struct {\n\tPath string\n}\n",
            ),
            (
                "src/net/http/httptest/server.go",
                "package httptest\n\ntype Server struct{}\n\nfunc NewServer(handler any) *Server { return nil }\n\nfunc (s *Server) Close() {}\n",
            ),
            (
                "mod/example.com/lib@v1.0.0/lib.go",
                "package lib\n\ntype Config struct {\n\tName string\n}\n",
            ),
        ],
    );
    use_roots(
        &mut a,
        Kind::Go,
        &[root.join("src"), root.join("mod/example.com/lib@v1.0.0")],
    );
    let mut d = |code: &str| {
        d_on(&mut a, "app/app.go", code);
        shown(&mut a)
    };
    let at = |file: &str, line: usize| format!("{}:{line}", root.join(file).display());
    assert_eq!(
        d("wg.Add"),
        jump(
            "Add \u{2192} WaitGroup.Add (via wg: WaitGroup)",
            &at("src/sync/waitgroup.go", 7)
        )
    );
    assert_eq!(
        d("t.Errorf"),
        jump(
            "Errorf \u{2192} common.Errorf (via t: T)",
            &at("src/testing/testing.go", 5)
        )
    );
    assert_eq!(
        d("ctx.Err"),
        jump(
            "Err \u{2192} Context.Err (via ctx: Context)",
            &at("src/context/context.go", 4)
        )
    );
    assert_eq!(
        d("r.URL"),
        jump(
            "URL \u{2192} Request.URL (via r: Request)",
            &at("src/net/http/request.go", 6)
        )
    );
    assert_eq!(
        d("r.URL.Path"),
        jump(
            "Path \u{2192} URL.Path (via r.URL: URL)",
            &at("src/net/url/url.go", 4)
        )
    );
    assert_eq!(
        d("srv.Close"),
        jump(
            "Close \u{2192} Server.Close (via httptest.NewServer() *Server)",
            &at("src/net/http/httptest/server.go", 7)
        )
    );
    assert_eq!(
        d("c.Name"),
        jump(
            "Name \u{2192} Config.Name (via c: Config)",
            &at("mod/example.com/lib@v1.0.0/lib.go", 4)
        )
    );
    assert_eq!(
        d("{New"),
        jump(
            "New \u{2192} Pool.New (via sync.Pool{\u{2026}})",
            &at("src/sync/pool.go", 4)
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

/// A Go local named `from` is a local: `from ` starts an import line in Python, not in Go, so the
/// package's `var from` in another file is not the answer (#521 review).
#[test]
fn a_go_local_named_from_is_the_local() {
    let (dir, mut a) = project_app(
        "go-local-from",
        &[
            ("go.mod", "module example.com/span\n"),
            (
                "main.go",
                "package main\n\nimport \"time\"\n\nfunc span(to time.Time) time.Duration {\n\tfrom := time.Now()\n\treturn to.Sub(from)\n}\n",
            ),
            ("other.go", "package main\n\nvar from = 1\n"),
        ],
    );
    a.external
        .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "main.go", "to.Sub(from");
    assert_eq!(
        shown(&mut a),
        jump("from \u{2192} span.from (local)", "main.go:6")
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Go's `pkg.X` ends at `pkg`'s directory only when that directory was read (#332): a chain
/// through a package's value, and a package the lookup cannot map, such as one committed under
/// `vendor/`, keep the search by name (#521 review).
#[test]
fn a_go_chain_or_a_vendored_package_keeps_the_search_by_name() {
    let (dir, mut a) = project_app(
        "go-chain-vendor",
        &[
            ("go.mod", "module example.com/app\n"),
            (
                "client/client.go",
                "package client\n\ntype Client struct{}\n\nvar Default = build()\n\nfunc build() *Client { return nil }\n\nfunc (c *Client) Do() {}\n",
            ),
            (
                "vendor/github.com/zzfake/errs/errs.go",
                "package errs\n\nfunc Wrap(e error) error { return e }\n",
            ),
            (
                "main.go",
                "package main\n\nimport (\n\t\"example.com/app/client\"\n\t\"github.com/zzfake/errs\"\n)\n\nfunc main() {\n\tclient.Default.Do()\n\t_ = errs.Wrap(nil)\n}\n",
            ),
        ],
    );
    a.external
        .insert(Kind::Go, (Vec::new(), Arc::new(Vec::new())));
    d_on(&mut a, "main.go", "client.Default.Do");
    assert_eq!(
        shown(&mut a),
        jump(
            "Do \u{2192} Client.Do (by name, 1 match)",
            "client/client.go:9"
        )
    );
    d_on(&mut a, "main.go", "errs.Wrap");
    assert_eq!(
        shown(&mut a),
        jump(
            "Wrap: by name, 1 match",
            "vendor/github.com/zzfake/errs/errs.go:3"
        )
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
