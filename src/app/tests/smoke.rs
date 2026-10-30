use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use clap::error::ErrorKind;
use clap::parser::ValueSource;

use crate::stats::ACTIONS;

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

const UNSMOKED: &[(u32, &str)] = &[
    (
        318,
        "a speed-up of d outside the project: the scenarios have no installed dependencies to time it on; the d bench measures it",
    ),
    (
        727,
        "Mermaid pictures: the scenarios play in tmux, which passes no kitty graphics, so merl draws the source there; src/mermaid/tests.rs and the layout tests cover the pictures",
    ),
    (
        769,
        "review's images side by side: tmux passes no kitty graphics, and without pictures images stay binary non-stops as before; src/ui/tests/picture.rs and the review stop test cover them",
    ),
    (
        801,
        "a theme's colours only, which the smoke screens (text) cannot see; tools/theme-bench compares them with LazyVim's",
    ),
    (
        796,
        "chrome colours only, which the smoke screens (text) cannot see; the theme bench's chrome and the_chrome_is_what_lazyvim_draws_in_every_ported_theme compare them with LazyVim's",
    ),
    (
        813,
        "the status path's colour only, which the smoke screens (text) cannot see; tests/snapshots hold it in tokyonight-moon and github-light",
    ),
    (
        810,
        "code colours only, which the smoke screens (text) cannot see; tools/theme-bench and tests/colors compare them with LazyVim's",
    ),
    (
        812,
        "the default theme's colours only, which the smoke screens (text) cannot see; tools/theme-bench compares them with LazyVim's",
    ),
    (
        834,
        "Mermaid pictures drawn by merman 0.9: tmux passes no kitty graphics, so merl draws the source there; src/mermaid/tests.rs covers the pictures",
    ),
    (
        1328,
        "TypeScript's class in d's list: every d of ts-d lands without a list; class_bench measures the class and a_definition_row_s_class_is_its_declaration_s_and_comes_before_a_differing_reason shows it",
    ),
    (
        1332,
        "Lua's class in d's list: no smoke scenario plays Lua; class_bench measures the class and a_definition_row_s_class_is_its_declaration_s_and_comes_before_a_differing_reason shows it",
    ),
    (
        1333,
        "SQL's class in d's list: no smoke scenario plays d in SQL; class_bench measures the class and a_definition_row_s_class_is_its_declaration_s_and_comes_before_a_differing_reason shows it",
    ),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn played_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for dir in std::fs::read_dir(root().join("tests/smoke/screens")).unwrap() {
        if let Ok(text) = std::fs::read_to_string(dir.unwrap().path().join("keys.txt")) {
            keys.extend(text.lines().map(String::from));
        }
    }
    keys
}

fn starts(path: &Path, out: &mut Vec<Vec<String>>) {
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let mut words = line
            .split_whitespace()
            .map(|w| w.trim_matches(['\'', '"']).to_owned());
        match words.next().as_deref() {
            Some("merl") => out.push(words.collect()),
            Some("include") => starts(&root().join(words.next().unwrap()), out),
            _ => {}
        }
    }
}

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
        let argv = std::iter::once("merl".to_owned()).chain(args.iter().cloned());
        match crate::Cli::command().try_get_matches_from(argv) {
            Ok(m) => ids.extend(
                m.ids()
                    .filter(|id| m.value_source(id.as_str()) == Some(ValueSource::CommandLine))
                    .map(ToString::to_string),
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
                .map_or_else(|| a.get_id().to_string(), |l| format!("--{l}"));
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

fn cited(text: &str) -> BTreeSet<u32> {
    regex::Regex::new(r"#(\d+)\b")
        .unwrap()
        .captures_iter(text)
        .map(|c| c[1].parse().unwrap())
        .collect()
}

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
                wrong.push(if issues.is_empty() {
                    format!("`{first}…` cites no issue")
                } else {
                    format!("`{first}…` cites {}, no scenario plays", issues.join(", "))
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
            let not_the_readme_whose_prose_cites_known_misses =
                path.file_name().is_some_and(|n| n != "README.md");
            let fixture = !path.starts_with(root().join("tests/smoke"))
                && not_the_readme_whose_prose_cites_known_misses;
            if path.is_dir() && fixture {
                dirs.push(path);
            } else if fixture || path.extension().is_some_and(|e| e == "steps") {
                played.extend(cited(&std::fs::read_to_string(&path).unwrap_or_default()));
            }
        }
    }
    let skipped: Vec<u32> = UNSMOKED.iter().map(|(n, _)| *n).collect();
    let wrong = unplayed(&with_changes(&changes()), &played, &skipped);
    assert!(
        wrong.is_empty(),
        "{}\nplay the feature in a tests/smoke scenario whose comment cites `(#N)`, or annotate a \
         fixture with it, or list the issue in UNSMOKED with why",
        wrong.join("\n")
    );
}

fn with_changes(changes: &[(String, String)]) -> String {
    let fragments: String = changes
        .iter()
        .map(|(_, text)| format!("\n{text}"))
        .collect();
    std::fs::read_to_string(root().join("CHANGELOG.md"))
        .unwrap()
        .replacen(
            "## [Unreleased]\n",
            &format!("## [Unreleased]\n{fragments}"),
            1,
        )
}

#[test]
fn a_change_s_feature_reaches_the_unreleased_guard() {
    let change = (
        "9.md".to_owned(),
        "### Added\n\n- A new key. (#9)\n".to_owned(),
    );
    let wrong = unplayed(&with_changes(&[change]), &BTreeSet::new(), &[]);
    assert!(wrong.iter().any(|w| w.contains("cites #9,")), "{wrong:?}");
}

#[test]
fn gather_changes_puts_each_heading_once_in_keep_a_changelog_s_order() {
    let dir = std::env::temp_dir().join(format!("merl-gather-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(dir.join("tools")).unwrap();
    std::fs::create_dir_all(dir.join("changes")).unwrap();
    std::fs::copy(
        root().join("tools/gather-changes.py"),
        dir.join("tools/gather-changes.py"),
    )
    .unwrap();
    for (name, text) in [
        ("12.md", "### Fixed\n\n- Twelve fixed. (#12)\n"),
        (
            "3.md",
            "### Fixed\n\n- Three fixed. (#3)\n\n### Added\n\n- Three added. (#3)\n",
        ),
    ] {
        std::fs::write(dir.join("changes").join(name), text).unwrap();
    }
    let run = std::process::Command::new("python3")
        .arg(dir.join("tools/gather-changes.py"))
        .arg("--print")
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "### Added\n\n- Three added. (#3)\n\n### Fixed\n\n- Three fixed. (#3)\n- Twelve fixed. \
         (#12)\n"
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

fn changes() -> Vec<(String, String)> {
    let Ok(dir) = std::fs::read_dir(root().join("changes")) else {
        return Vec::new();
    };
    dir.map(|f| {
        let path = f.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        (name, std::fs::read_to_string(&path).unwrap_or_default())
    })
    .collect()
}

const HEADINGS: [&str; 6] = [
    "Added",
    "Changed",
    "Deprecated",
    "Removed",
    "Fixed",
    "Security",
];

fn misfiled(name: &str, text: &str) -> Option<String> {
    let number = name
        .strip_suffix(".md")
        .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    let Some(n) = number.and_then(|n| n.parse::<u32>().ok()) else {
        return Some(format!("changes/{name}: name it changes/N.md, N the issue"));
    };
    if !text.starts_with("### ") {
        return Some(format!(
            "changes/{name}: start with its heading, `### Fixed`"
        ));
    }
    for section in format!("\n{text}").split("\n### ").skip(1) {
        let heading = section.lines().next().unwrap_or("").trim();
        if !HEADINGS.contains(&heading) {
            return Some(format!(
                "changes/{name}: `### {heading}` is none of Keep a Changelog's {}",
                HEADINGS.join(", ")
            ));
        }
        let entries: Vec<&str> = section.split("\n- ").skip(1).collect();
        if entries.is_empty() || entries.iter().any(|e| !cited(e).contains(&n)) {
            return Some(format!(
                "changes/{name}: every entry under `### {heading}` cites (#{n})"
            ));
        }
    }
    None
}

#[test]
fn every_change_is_an_issue_s_entry_under_a_keep_a_changelog_heading() {
    let wrong: Vec<String> = changes()
        .iter()
        .filter_map(|(name, text)| misfiled(name, text))
        .collect();
    assert!(
        wrong.is_empty(),
        "{}\nAGENTS.md `## Working on an issue` step 5 has the form",
        wrong.join("\n")
    );
}

#[test]
fn a_change_is_misfiled_by_its_name_its_heading_or_its_issue() {
    let fixed = "### Fixed\n\n- `d` lands. (#7)\n";
    assert_eq!(misfiled("7.md", fixed), None);
    assert_eq!(
        misfiled(
            "7.md",
            "### Added\n\n- A. (#7)\n- B.\n  (#3, #7)\n\n### Fixed\n\n- C (#7).\n"
        ),
        None
    );
    for name in ["fix.md", "7.txt", ".md", "+7.md", "7"] {
        assert!(misfiled(name, fixed).is_some(), "{name}");
    }
    for text in [
        "- `d` lands. (#7)\n",
        "### Fixes\n\n- `d` lands. (#7)\n",
        "### Fixed\n",
        "### Fixed\n\n- `d` lands. (#8)\n",
        "### Fixed\n\n- `d` lands. (#7)\n- `u` too.\n",
        "### Fixed\n\n- `d` lands. (#7)\n\n### Fixd\n\n- `u` too. (#7)\n",
    ] {
        assert!(misfiled("7.md", text).is_some(), "{text}");
    }
}

fn released(changelog: &str) -> Vec<(String, String)> {
    let mut sections: Vec<(String, String)> = Vec::new();
    for line in changelog.lines() {
        if line.starts_with('[') && line.contains("]: ") {
            break;
        }
        if let Some(heading) = line.strip_prefix("## [") {
            let version = heading.split(']').next().unwrap_or("").to_owned();
            sections.push((version, String::new()));
        }
        if let Some((_, text)) = sections.last_mut() {
            text.push_str(line);
            text.push('\n');
        }
    }
    sections.retain(|(version, _)| version != "Unreleased");
    sections
}

fn changed_releases(at_release: &str, now: &str) -> Vec<String> {
    let then = released(at_release);
    released(now)
        .into_iter()
        .filter(|s| then.iter().any(|t| t.0 == s.0) && !then.contains(s))
        .map(|(version, _)| version)
        .collect()
}

fn git(args: &[&str]) -> String {
    git_in(&root(), args)
}

fn git_in(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn last_release_on_master(dir: &Path) -> String {
    let base = git_in(dir, &["merge-base", "HEAD", "origin/master"]);
    let log = git_in(dir, &["log", "--format=%H %s", base.trim()]);
    log.lines()
        .filter_map(|l| l.split_once(' '))
        .find(|(_, subject)| subject.starts_with("release: "))
        .map(|(hash, _)| hash.to_owned())
        .expect("no `release: X.Y.Z` commit in the history: a shallow clone needs fetch-depth: 0")
}

#[test]
fn released_changelog_sections_stay_as_released() {
    let release = last_release_on_master(&root());
    let at_release = git(&["show", &format!("{release}:CHANGELOG.md")]);
    let now = std::fs::read_to_string(root().join("CHANGELOG.md")).unwrap();
    let wrong = changed_releases(&at_release, &now);
    assert!(
        wrong.is_empty(),
        "CHANGELOG.md's [{}] differ from release commit {release}: a merge of master after a \
         release moves a branch's entries under the released heading; move them back to \
         [Unreleased]",
        wrong.join("], [")
    );
}

#[test]
fn a_release_branch_keeps_its_own_section_open_until_master_has_it() {
    let dir = std::env::temp_dir().join(format!("merl-last-release-{}", std::process::id()));
    drop(std::fs::remove_dir_all(&dir));
    std::fs::create_dir_all(&dir).unwrap();
    let git = |args: &[&str]| {
        let mut all = vec!["-c", "user.email=t@t", "-c", "user.name=t"];
        all.extend_from_slice(args);
        git_in(&dir, &all).trim().to_owned()
    };
    git(&["init", "-q", "-b", "master"]);
    git(&["commit", "-q", "--allow-empty", "-m", "release: 0.1.0 (#1)"]);
    let released = git(&["rev-parse", "HEAD"]);
    git(&["update-ref", "refs/remotes/origin/master", "HEAD"]);
    git(&["switch", "-q", "-c", "release/0.2.0"]);
    git(&["commit", "-q", "--allow-empty", "-m", "release: 0.2.0"]);
    let releasing = git(&["rev-parse", "HEAD"]);
    git(&["commit", "-q", "--allow-empty", "-m", "fix: a panic"]);
    assert_eq!(last_release_on_master(&dir), released);
    git(&["update-ref", "refs/remotes/origin/master", "HEAD"]);
    assert_eq!(last_release_on_master(&dir), releasing);
    drop(std::fs::remove_dir_all(&dir));
}

#[test]
fn a_released_section_counts_as_changed_by_any_line_but_a_new_one_does_not() {
    let then = "## [Unreleased]\n\n## [0.2.0] - 2026-01-02\n\n- B. (#2)\n\n\
                ## [0.1.0] - 2026-01-01\n\n- A. (#1)\n\n[0.2.0]: url\n";
    let now = "## [Unreleased]\n\n- D. (#4)\n\n## [0.3.0] - 2026-01-03\n\n- C. (#3)\n\n\
               ## [0.2.0] - 2026-01-02\n\n- B. (#2)\n- D. (#4)\n\n\
               ## [0.1.0] - 2026-01-01\n\n- A. (#1)\n\n[0.3.0]: url\n[0.2.0]: url\n";
    assert_eq!(changed_releases(then, now), ["0.2.0"]);
    assert_eq!(changed_releases(then, then), [] as [String; 0]);
}
