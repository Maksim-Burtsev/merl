//! `d` in Go.

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
        (
            "globals.go",
            "defaultRepo.DeleteUser|(21",
            picker("DeleteUser: by name, 2 declarations", &BOTH_DELETE_USER),
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
    // Two aliases of each other, which no compiler accepts, end.
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
    assert_eq!(a.message, "Run \u{2192} C.Run (by name, 1 match)");
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
    // `GOFLAGS=-tags=fast` builds the other one.
    let mut a = fixture_app("go");
    a.go_build.tags = vec!["fast".into()];
    d_on(&mut a, "platforms.go", "codec.Encode");
    assert_eq!(
        shown(&mut a),
        jump(
            "Encode \u{2192} Codec.Encode (via codec: Codec)",
            "platform/codec_fast.go:8",
        )
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
        // `meter_fast.go` (`// +build`) is not known to be built, so `meter_windows.go` loses
        // to nothing.
        d_on(&mut a, "platforms_gate.go", "meter.Sample");
        assert_eq!(
            shown(&mut a),
            both(
                "Sample: by name, 2 declarations",
                "Meter.Sample",
                "platform/meter_fast.go:8",
                "platform/meter_windows.go:5",
            )
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
