//! The smoke scenarios (`tests/smoke/*.steps`) play every action of `KEYS` and every flag of the
//! command line, or `NOT_SMOKED` says why not (#310), as `NOT_TAUGHT` does for the tutor.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use clap::error::ErrorKind;
use clap::parser::ValueSource;

use crate::stats::ACTIONS;

/// Actions of `KEYS`, and flags as `--long`, that no scenario plays, each with why. A new row in
/// `KEYS` or a new flag fails the test below until a scenario plays it or it is listed here.
const NOT_SMOKED: &[(&str, &str)] = &[(
    "--version",
    "its text changes with every release; run.py asks both builds for it before any play",
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

/// The ids of the arguments the scenarios start merl with, read by clap as merl reads them.
fn played_flags() -> BTreeSet<String> {
    let mut lines = Vec::new();
    for f in std::fs::read_dir(root().join("tests/smoke")).unwrap() {
        let path = f.unwrap().path();
        if path.extension().is_some_and(|e| e == "steps") {
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
