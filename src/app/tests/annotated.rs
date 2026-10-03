//! `d` over the annotated fixtures (#307). A comment under a line, with a caret under a word,
//! says what `d` on that word shows; `tests/fixtures/README.md` has the grammar. One test walks
//! every fixture, presses `d` on every annotation and reports every failure at once.

use super::*;

/// The comment markers an annotation may start with: every kind's line comment.
const MARKERS: [&str; 6] = ["//", "#", "--", ";", "%", "(*"];

/// What an annotation wants `d` to show.
#[derive(Debug)]
enum Want {
    Jump(String),
    /// The rows, and whether more may follow (`, …`).
    Picker(Vec<String>, bool),
    None,
    NotJump,
}

/// What `d` showed.
#[derive(Debug)]
enum Got {
    Jump(String),
    Picker(Vec<String>),
    None,
}

struct Case {
    fixture: String,
    /// The file, from the fixture's root.
    file: String,
    /// The annotation's own line, and the probed line above it, both from 1.
    at: usize,
    line: usize,
    col: usize,
    want: Result<Want, String>,
    /// The annotation as written, after `d:`.
    raw: String,
    status: Option<String>,
}

/// `(caret column, text after the caret)` of an annotation line: a line comment that is only a
/// caret and what follows it. The text should start with `d:`; a typo there fails its case
/// rather than dropping it unseen.
fn annotation(line: &str) -> Option<(usize, &str)> {
    let caret = line.find('^')?;
    if !MARKERS.contains(&line[..caret].trim()) {
        return None;
    }
    Some((
        caret,
        line[caret + 1..].trim().trim_end_matches("*)").trim(),
    ))
}

/// The text of a `status:` line.
fn status_line(line: &str) -> Option<&str> {
    let body = line.trim_start();
    let body = MARKERS.iter().find_map(|m| body.strip_prefix(m))?;
    Some(
        body.trim_start()
            .strip_prefix("status:")?
            .trim()
            .trim_end_matches("*)")
            .trim(),
    )
}

fn parse_answer(s: &str) -> Result<Want, String> {
    let s = s.trim();
    Ok(match s {
        "none" => Want::None,
        "!jump" => Want::NotJump,
        _ => match s.strip_prefix("picker ") {
            Some(rows) => {
                let (rows, more) = match rows.trim_end().strip_suffix('…') {
                    Some(r) => (r.trim_end().trim_end_matches(','), true),
                    None => (rows, false),
                };
                let rows: Vec<String> = rows.split(',').map(|r| r.trim().to_owned()).collect();
                if rows.iter().any(|r| !is_place(r)) {
                    return Err(format!("bad picker rows `{s}`"));
                }
                Want::Picker(rows, more)
            }
            None if is_place(s) => Want::Jump(s.to_owned()),
            None => return Err(format!("bad answer `{s}`")),
        },
    })
}

fn is_place(s: &str) -> bool {
    s.rsplit_once(':')
        .is_some_and(|(f, n)| !f.is_empty() && !f.contains(' ') && n.parse::<usize>().is_ok())
}

/// `today; want wanted (#N)`: today's answer is checked, the wanted one only has to parse.
fn parse_want(s: &str) -> Result<Want, String> {
    let Some((today, wanted)) = s.split_once("; want ") else {
        return parse_answer(s);
    };
    let issue = wanted
        .rsplit_once("(#")
        .and_then(|(w, n)| Some((w, n.strip_suffix(')')?.parse::<u32>().ok()?)));
    let Some((wanted, _)) = issue else {
        return Err(format!("`; want` without an issue `(#N)`: `{wanted}`"));
    };
    parse_answer(wanted)?;
    parse_answer(today)
}

fn cases() -> Vec<Case> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut cases = Vec::new();
    let mut fixtures: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    fixtures.sort();
    for fixture in fixtures {
        let (_, files) = crate::tree::build(&root.join(&fixture), false);
        for file in files {
            let path = root.join(&fixture).join(&file);
            if search::kind_of(&path).is_none() {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let lines: Vec<&str> = text.lines().collect();
            for (i, l) in lines.iter().enumerate() {
                let Some((col, want)) = annotation(l) else {
                    continue;
                };
                // The probed line is the nearest above that is no annotation of its own.
                let line = (0..i)
                    .rev()
                    .find(|&j| annotation(lines[j]).is_none() && status_line(lines[j]).is_none())
                    .map_or(0, |j| j + 1);
                cases.push(Case {
                    fixture: fixture.clone(),
                    file: file.to_string_lossy().into_owned(),
                    at: i + 1,
                    line,
                    col,
                    want: match want.strip_prefix("d:") {
                        Some(w) => parse_want(w.trim()),
                        None => Err(format!("no `d:` after the caret: `{want}`")),
                    },
                    raw: want.strip_prefix("d:").unwrap_or(want).trim().to_owned(),
                    status: lines
                        .get(i + 1)
                        .and_then(|n| status_line(n))
                        .map(str::to_owned),
                });
            }
        }
    }
    cases
}

/// Presses `d` where `case` points and reads what it showed, with the status line.
fn play(case: &Case) -> (Got, String) {
    let mut a = fixture_app(&case.fixture);
    let path = a.root.join(&case.file);
    a.jump_to(&path, case.line);
    a.col = case.col;
    let before = at(&a);
    press(&mut a, KeyCode::Char('d'), KeyModifiers::NONE);
    let status = a.message.clone();
    let got = match a.picker {
        Some(_) => Got::Picker(definition_rows(&mut a).into_iter().map(|r| r.2).collect()),
        // Nothing found leaves the cursor where it was and says so, or names the label the word
        // is (#315) or a builtin with no source (#336); a jump may land on the line it started
        // from, on a declaration found by name.
        None if at(&a) == before
            && (status.starts_with("no ")
                || status.ends_with(": argument label")
                || status.ends_with(": key")
                || status.contains(": builtin, no source")) =>
        {
            Got::None
        }
        None => {
            let (path, line) = at(&a);
            let place = path.strip_prefix(&a.root).unwrap_or(&path);
            Got::Jump(format!("{}:{}", place.display(), line + 1))
        }
    };
    (got, status)
}

fn matches(want: &Want, got: &Got) -> bool {
    match (want, got) {
        (Want::Jump(w), Got::Jump(g)) => w == g,
        (Want::None, Got::None) | (Want::NotJump, Got::None | Got::Picker(_)) => true,
        (Want::Picker(w, more), Got::Picker(g)) => {
            w.iter().all(|r| g.contains(r)) && (*more || g.iter().all(|r| w.contains(r)))
        }
        _ => false,
    }
}

/// `got` in the annotation's own grammar, ready to paste.
fn written(got: &Got) -> String {
    match got {
        Got::Jump(place) => place.clone(),
        Got::Picker(rows) => format!("picker {}", rows.join(", ")),
        Got::None => "none".into(),
    }
}

/// #307. Every annotation in `tests/fixtures` holds: the failures are listed together, each as
/// `fixture/file:line`, what it wants and what `d` showed.
#[test]
fn d_answers_every_annotation_in_the_fixtures() {
    let cases = cases();
    assert!(!cases.is_empty(), "no annotations in tests/fixtures");
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = cases.len().div_ceil(threads);
    let failures: Vec<String> = std::thread::scope(|s| {
        let runs: Vec<_> = cases
            .chunks(chunk)
            .map(|part| s.spawn(move || part.iter().filter_map(check).collect::<Vec<_>>()))
            .collect();
        runs.into_iter().flat_map(|r| r.join().unwrap()).collect()
    });
    assert!(
        failures.is_empty(),
        "{} of {} annotations fail:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

/// The failure of one case, if it fails.
fn check(case: &Case) -> Option<String> {
    let here = format!("tests/fixtures/{}/{}:{}", case.fixture, case.file, case.at);
    let want = match &case.want {
        Ok(want) => want,
        Err(e) => return Some(format!("{here}: {e}")),
    };
    // A panic is one failure among the others, not the end of the report.
    let Ok((got, status)) = std::panic::catch_unwind(|| play(case)) else {
        return Some(format!("{here}: `d` panicked"));
    };
    let status_ok = case
        .status
        .as_ref()
        .is_none_or(|s| status.contains(s.as_str()));
    (!matches(want, &got) || !status_ok).then(|| {
        format!(
            "{here}: want `d: {}`{}, got `d: {}` ({status})",
            case.raw,
            case.status
                .as_ref()
                .map_or(String::new(), |s| format!(" with status `{s}`")),
            written(&got)
        )
    })
}

#[test]
fn the_annotation_grammar() {
    assert_eq!(annotation("    //   ^ d: a.go:3"), Some((9, "d: a.go:3")));
    assert_eq!(annotation("\t# ^ d: none"), Some((3, "d: none")));
    assert_eq!(annotation("# ^ D: none"), Some((2, "D: none")));
    assert_eq!(annotation("    %  ^ d: a.erl:3"), Some((7, "d: a.erl:3")));
    assert_eq!(annotation("x = 1  # ^ d: none"), None);
    assert_eq!(annotation("-- a ^ d: none"), None);
    assert!(matches!(parse_want("none"), Ok(Want::None)));
    assert!(matches!(parse_want("!jump"), Ok(Want::NotJump)));
    assert!(matches!(parse_want("a/b.py:3"), Ok(Want::Jump(p)) if p == "a/b.py:3"));
    assert!(matches!(
        parse_want("picker a.py:1, b.py:2, …"),
        Ok(Want::Picker(r, true)) if r == ["a.py:1", "b.py:2"]
    ));
    assert!(matches!(
        parse_want("none; want a.py:1 (#12)"),
        Ok(Want::None)
    ));
    assert!(parse_want("none; want a.py:1").is_err());
    assert!(parse_want("somewhere").is_err());
    assert_eq!(status_line("  # status: via import"), Some("via import"));
    let pick = |r: &[&str]| Got::Picker(r.iter().map(|s| s.to_string()).collect());
    let want = |r: &[&str], more| Want::Picker(r.iter().map(|s| s.to_string()).collect(), more);
    assert!(matches(&want(&["a:1"], true), &pick(&["a:1", "b:2"])));
    assert!(!matches(&want(&["a:1"], false), &pick(&["a:1", "b:2"])));
    assert!(!matches(
        &want(&["a:1", "c:3"], true),
        &pick(&["a:1", "b:2"])
    ));
    assert!(matches(&Want::NotJump, &pick(&["a:1"])));
    assert!(!matches(&Want::NotJump, &Got::Jump("a:1".into())));
}
