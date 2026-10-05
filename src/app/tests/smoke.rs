//! The smoke scenarios (`tests/smoke/*.steps`) play every action of `KEYS` and every flag of the
//! command line, or `NOT_SMOKED` says why not (#310), as `NOT_TAUGHT` does for the tutor. Every
//! feature `## [Unreleased]` adds or changes plays in a scenario too, or `UNSMOKED` says why (#551).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use clap::error::ErrorKind;
use clap::parser::ValueSource;

use crate::stats::ACTIONS;

/// Actions of `KEYS`, and flags as `--long`, that no scenario plays, each with why. A new row in
/// `KEYS` or a new flag fails the test below until a scenario plays it or it is listed here.
const NOT_SMOKED: &[(&str, &str)] = &[
    (
        "--version",
        "its text changes with every release; run.py asks both builds for it before any play",
    ),
    (
        "--for-agents",
        "prints a guide for an agent and exits, no screen to play; hidden while #599 is an experiment",
    ),
];

const UNSMOKED: &[(u32, &str)] = &[(
    318,
    "a speed-up of d outside the project: the scenarios have no installed dependencies to time it on; the d bench measures it",
)];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The actions merl counted a press of in the scenarios CI plays: the `keys.txt` beside each
/// one's screens, which `tests/smoke/run.py --update` writes and `--golden` checks.
fn played_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for dir in std::fs::read_dir(root().join("tests/smoke/screens")).unwrap() {
        if let Ok(text) = std::fs::read_to_string(dir.unwrap().path().join("keys.txt")) {
            keys.extend(text.lines().map(String::from));
        }
    }
    keys
}

/// The arguments of the `merl ARGS` steps of a scenario and of the files it includes.
fn starts(path: &Path, out: &mut Vec<Vec<String>>) {
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let mut words = line
            .split_whitespace()
            .map(|w| w.trim_matches(['\'', '"']).to_string());
        match words.next().as_deref() {
            Some("merl") => out.push(words.collect()),
            Some("include") => starts(&root().join(words.next().unwrap()), out),
            _ => {}
        }
    }
}

/// The ids of the arguments the scenarios CI plays start merl with, read by clap as merl reads
/// them. CI's scenarios are the ones with screens checked in: `RELEASE_ONLY` in `run.py` has
/// none, and `--golden` fails on a screens folder with no scenario.
fn played_flags() -> BTreeSet<String> {
    let mut lines = Vec::new();
    for f in std::fs::read_dir(root().join("tests/smoke")).unwrap() {
        let path = f.unwrap().path();
        let played = path
            .file_stem()
            .is_some_and(|name| root().join("tests/smoke/screens").join(name).is_dir());
        if path.extension().is_some_and(|e| e == "steps") && played {
            starts(&path, &mut lines);
        }
    }
    let mut ids = BTreeSet::new();
    for args in lines {
        let argv = std::iter::once("merl".to_string()).chain(args.iter().cloned());
        match crate::Cli::command().try_get_matches_from(argv) {
            Ok(m) => ids.extend(
                m.ids()
                    .filter(|id| m.value_source(id.as_str()) == Some(ValueSource::CommandLine))
                    .map(|id| id.to_string()),
            ),
            Err(e) if e.kind() == ErrorKind::DisplayHelp => _ = ids.insert("help".into()),
            Err(e) if e.kind() == ErrorKind::DisplayVersion => _ = ids.insert("version".into()),
            Err(e) => panic!("a scenario starts `merl {}`: {e}", args.join(" ")),
        }
    }
    ids
}

#[test]
fn every_key_and_flag_is_smoked_or_skipped_on_purpose() {
    let keys = played_keys();
    let ids = played_flags();
    let mut cli = crate::Cli::command();
    cli.build();
    let flags: Vec<(String, bool)> = cli
        .get_arguments()
        .map(|a| {
            let name = a
                .get_long()
                .map_or(a.get_id().to_string(), |l| format!("--{l}"));
            (name, ids.contains(a.get_id().as_str()))
        })
        .collect();
    let all = ACTIONS
        .iter()
        .map(|a| (a.name.clone(), keys.contains(&a.name)))
        .chain(flags);
    let mut wrong = Vec::new();
    let mut known = Vec::new();
    for (name, played) in all {
        let skipped = NOT_SMOKED.iter().any(|(n, _)| *n == name);
        match (played, skipped) {
            (false, false) => wrong.push(format!("`{name}` is played by no scenario")),
            (true, true) => wrong.push(format!("`{name}` is played, and listed in NOT_SMOKED")),
            _ => {}
        }
        known.push(name);
    }
    for (name, _) in NOT_SMOKED {
        if !known.iter().any(|k| k == name) {
            wrong.push(format!(
                "NOT_SMOKED lists `{name}`, no action of KEYS nor a flag"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{}\nplay it in a tests/smoke scenario (AGENTS.md says where) and write its keys.txt with \
         `tests/smoke/run.py --update --only NAME`, or list it in NOT_SMOKED with why",
        wrong.join("\n")
    );
}

/// The issues `text` cites as `#N`.
fn cited(text: &str) -> BTreeSet<u32> {
    regex::Regex::new(r"#(\d+)\b")
        .unwrap()
        .captures_iter(text)
        .map(|c| c[1].parse().unwrap())
        .collect()
}

/// The `### Added` and `### Changed` entries of the changelog's `## [Unreleased]` that cite no
/// issue in `played`, nor one listed in `skipped`.
fn unplayed(changelog: &str, played: &BTreeSet<u32>, skipped: &[u32]) -> Vec<String> {
    let unreleased = changelog
        .split("## [")
        .find(|s| s.starts_with("Unreleased]"))
        .unwrap_or("");
    let mut wrong = Vec::new();
    for section in unreleased.split("\n### ").skip(1) {
        if !(section.starts_with("Added") || section.starts_with("Changed")) {
            continue;
        }
        for entry in section.split("\n- ").skip(1) {
            let issues = cited(entry);
            if !issues
                .iter()
                .any(|n| played.contains(n) || skipped.contains(n))
            {
                let first: String = entry
                    .lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(60)
                    .collect();
                let issues: Vec<String> = issues.iter().map(|n| format!("#{n}")).collect();
                wrong.push(match issues.is_empty() {
                    true => format!("`{first}…` cites no issue"),
                    false => format!("`{first}…` cites {}, no scenario plays", issues.join(", ")),
                });
            }
        }
    }
    wrong
}

#[test]
fn every_unreleased_feature_is_smoked_or_skipped_on_purpose() {
    let mut played = BTreeSet::new();
    let mut dirs = vec![root().join("tests/smoke"), root().join("tests/fixtures")];
    while let Some(dir) = dirs.pop() {
        for f in std::fs::read_dir(dir).unwrap() {
            let path = f.unwrap().path();
            // The fixtures' README cites issues in prose, some of them as known misses: no play.
            let fixture = !path.starts_with(root().join("tests/smoke"))
                && path.file_name().is_some_and(|n| n != "README.md");
            if path.is_dir() && fixture {
                dirs.push(path);
            } else if fixture || path.extension().is_some_and(|e| e == "steps") {
                played.extend(cited(&std::fs::read_to_string(&path).unwrap_or_default()));
            }
        }
    }
    let changelog = std::fs::read_to_string(root().join("CHANGELOG.md")).unwrap();
    let skipped: Vec<u32> = UNSMOKED.iter().map(|(n, _)| *n).collect();
    let wrong = unplayed(&changelog, &played, &skipped);
    assert!(
        wrong.is_empty(),
        "{}\nplay the feature in a tests/smoke scenario whose comment cites `(#N)`, or annotate a \
         fixture with it, or list the issue in UNSMOKED with why",
        wrong.join("\n")
    );
}

#[test]
fn an_entry_passes_by_any_issue_it_cites_that_a_scenario_plays() {
    let log = "## [Unreleased]\n\n### Added\n\n- A, played. (#1, #2)\n- B, skipped. (#3)\n- C, \
               not played.\n  (#4)\n- D, no issue.\n\n### Fixed\n\n- E, a fix. (#5)\n\n\
               ## [0.1.0] - 2026-01-01\n\n### Added\n\n- F, released. (#6)\n";
    let wrong = unplayed(log, &BTreeSet::from([2]), &[3]);
    assert_eq!(wrong.len(), 2, "{wrong:?}");
    assert!(wrong[0].contains("cites #4,") && wrong[1].contains("cites no issue"));
}
