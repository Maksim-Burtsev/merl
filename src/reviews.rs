//! Review stats (#242): a line per `--review` session in `~/.local/state/merl/reviews.tsv`, next
//! to `keys.tsv`, which `merl --reviews` prints. Nothing on screen while you review.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::app::plural;
use crate::git;
use crate::stats::{self, WINDOW};

/// A gap between presses counts in full up to this, a longer one as this: a review left open
/// for hours is not hours of review.
const IDLE: Duration = Duration::from_secs(5 * 60);

/// The file's first line: what each column of a session holds. The times are in seconds, `back`
/// counts `[`, `stops` the hunks `c` / `C` stopped on, `last_hunk` is 1 when `c` reached `last
/// hunk of the review`.
const HEAD: &str = "date\trepo\tbranch\tround\tfiles\thunks\tadded\tdeleted\ton_review_s\t\
                    elsewhere_s\texcursions\tjumps\tback\tstops\tviewed\tlast_hunk";

/// Next to `keys.tsv`.
pub fn path() -> Option<PathBuf> {
    Some(stats::path()?.with_file_name("reviews.tsv"))
}

/// The open file, the cursor's line in it, and whether the file is one of the review's.
pub type Spot = (Option<PathBuf>, usize, bool);

/// A hunk of the review, as the status bar's `hunk i/n` numbers it: the file relative to the
/// root and the hunk's place in it, from 1. A reload that shifts the lines keeps the hunk.
pub type Stop = (PathBuf, usize);

/// What one review session did, counted on each press.
pub struct Session {
    /// The repository and the branch the session is of, taken when the review opened: a
    /// `git switch` during it does not file it under another branch.
    pub repo: String,
    pub branch: String,
    /// The last press, or the start of the review.
    last: Instant,
    /// Left the review by a jump and not back on one of its files yet.
    out: bool,
    pub presses: u64,
    on_review: Duration,
    elsewhere: Duration,
    /// Jumps (`d`, `u`, `D`, a picker's Enter) from a file of the review to a file outside it,
    /// and the jumps outside they took, the first included.
    excursions: u64,
    jumps: u64,
    /// `[` presses.
    back: u64,
    stops: HashSet<Stop>,
    last_hunk: bool,
}

impl Session {
    /// The review of `branch` in `repo` opened at `at`, on the hunk `stop` when it opened on one.
    pub fn new(at: Instant, repo: String, branch: String, stop: Option<Stop>) -> Self {
        Self {
            repo,
            branch,
            last: at,
            out: false,
            presses: 0,
            on_review: Duration::ZERO,
            elsewhere: Duration::ZERO,
            excursions: 0,
            jumps: 0,
            back: 0,
            stops: HashSet::from_iter(stop),
            last_hunk: false,
        }
    }

    /// A press at `at`, the cursor at `from` when it came: the time since the last one goes
    /// there, [`IDLE`] at most.
    pub fn pressed(&mut self, at: Instant, from: &Spot) {
        let gap = at.saturating_duration_since(self.last).min(IDLE);
        let total = match from.2 {
            true => &mut self.on_review,
            false => &mut self.elsewhere,
        };
        *total += gap;
        (self.last, self.presses) = (at, self.presses + 1);
    }

    /// `action` (a `KEYS` action) took the cursor from `from` to `to`; `stop`: `c` / `C` left it
    /// on that hunk; `end`: it was `c` saying `last hunk of the review`.
    pub fn moved(
        &mut self,
        action: Option<&str>,
        from: &Spot,
        to: &Spot,
        stop: Option<Stop>,
        end: bool,
    ) {
        let jump = from != to && matches!(action, Some("d" | "u" | "D" | "Picker: Enter"));
        if jump && from.2 && !to.2 {
            self.excursions += 1;
            self.out = true;
        }
        if jump && self.out && !to.2 {
            self.jumps += 1;
        }
        self.out &= !to.2;
        self.back += u64::from(action == Some("["));
        self.stops.extend(stop);
        self.last_hunk |= end;
    }

    /// The session's columns from `files` on, as [`HEAD`] names them. `hunks` are those of each
    /// file of the review as it is now; a stop on a hunk the file no longer has does not count,
    /// so the hunks stopped on are never more than the hunks.
    pub fn columns(
        &self,
        review: &git::Review,
        hunks: &HashMap<PathBuf, usize>,
        viewed: usize,
    ) -> String {
        let lines = |f: fn(&git::ReviewFile) -> usize| review.files.iter().map(f).sum::<usize>();
        let stops = (self.stops.iter())
            .filter(|(path, i)| hunks.get(path).is_some_and(|n| i <= n))
            .count();
        [
            review.files.len(),
            hunks.values().sum(),
            lines(|f| f.added),
            lines(|f| f.deleted),
            self.on_review.as_secs() as usize,
            self.elsewhere.as_secs() as usize,
            self.excursions as usize,
            self.jumps as usize,
            self.back as usize,
            stops,
            viewed,
            usize::from(self.last_hunk),
        ]
        .map(|n| n.to_string())
        .join("\t")
    }
}

/// Adds a session to the file under `today`, as the next round of `branch` in `repo`, and drops
/// the sessions older than [`WINDOW`] days. Written as `keys.tsv` is: read again right before
/// and replaced whole, so parallel merls keep each other's sessions. A file that cannot be read
/// is left as it is.
pub fn add(path: &Path, today: i64, repo: &str, branch: &str, columns: &str) -> Result<()> {
    let clean = |s: &str| s.replace(['\t', '\n'], " ");
    let (repo, branch) = (clean(repo), clean(branch));
    let text = stats::read(path)?;
    let kept: Vec<&str> = text.lines().filter(|l| recent(l, today)).collect();
    let round = 1 + kept
        .iter()
        .filter(|l| {
            l.split('\t')
                .skip(1)
                .take(2)
                .eq([repo.as_str(), branch.as_str()])
        })
        .count();
    let mut out = format!("{HEAD}\n");
    for line in kept {
        _ = writeln!(out, "{line}");
    }
    _ = writeln!(
        out,
        "{}\t{repo}\t{branch}\t{round}\t{columns}",
        stats::date(today)
    );
    stats::write(path, &out)
}

/// A session of the last [`WINDOW`] days; the head is none.
fn recent(line: &str, today: i64) -> bool {
    (line.split('\t').next())
        .and_then(stats::day)
        .is_some_and(|day| today - day < WINDOW)
}

/// `merl --reviews`.
pub fn report(path: &Path, today: i64) -> Result<String> {
    Ok(table(&stats::read(path)?, today))
}

/// `9:05`: minutes and seconds.
fn clock(secs: u64) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// The sessions of the last [`WINDOW`] days, newest first, then how many and the median active
/// time of a first round and of a later one.
fn table(text: &str, today: i64) -> String {
    let head = [
        "date",
        "branch",
        "round",
        "files",
        "hunks",
        "lines",
        "active",
        "on hunks",
        "excursions",
    ];
    let mut cells = vec![head.map(String::from)];
    let (mut first, mut later) = (Vec::new(), Vec::new());
    for line in text.lines().rev().filter(|l| recent(l, today)) {
        let cols: Vec<&str> = line.split('\t').collect();
        let nums = cols.get(3..11).and_then(|n| {
            n.iter()
                .map(|n| n.parse::<u64>().ok())
                .collect::<Option<Vec<_>>>()
        });
        let Some([round, files, hunks, added, deleted, on, off, excursions]) =
            nums.and_then(|n| <[u64; 8]>::try_from(n).ok())
        else {
            continue;
        };
        let rounds = if round == 1 { &mut first } else { &mut later };
        rounds.push(on + off);
        cells.push([
            cols[0].to_string(),
            cols[2].to_string(),
            round.to_string(),
            files.to_string(),
            hunks.to_string(),
            (added + deleted).to_string(),
            clock(on + off),
            clock(on),
            excursions.to_string(),
        ]);
    }
    let n = cells.len() - 1;
    let mut summary = format!("{WINDOW} days: {n} session{}", plural(n));
    for (what, times) in [("median first round", first), ("later rounds", later)] {
        if let Some(m) = median(times) {
            _ = write!(summary, " · {what} {}", clock(m));
        }
    }
    if n == 0 {
        return summary + "\n";
    }
    let widths: Vec<usize> = (0..head.len())
        .map(|i| {
            cells
                .iter()
                .map(|c| c[i].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut out = String::new();
    for row in &cells {
        let line: Vec<String> = row
            .iter()
            .zip(&widths)
            .enumerate()
            .map(|(i, (cell, &w))| match i {
                0 | 1 => format!("{cell:<w$}"),
                _ => format!("{cell:>w$}"),
            })
            .collect();
        _ = writeln!(out, "{}", line.join("  ").trim_end());
    }
    format!("{out}\n{summary}\n")
}

fn median(mut times: Vec<u64>) -> Option<u64> {
    times.sort_unstable();
    let n = times.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(times[n / 2]),
        _ => Some((times[n / 2 - 1] + times[n / 2]) / 2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("merl-reviews-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("state/merl/reviews.tsv")
    }

    fn spot(file: &str, line: usize, on: bool) -> Spot {
        (Some(PathBuf::from(file)), line, on)
    }

    fn session(at: Instant) -> Session {
        Session::new(at, "merl".into(), "feat".into(), None)
    }

    /// A gap up to five minutes counts in full, a longer one as five minutes, and it goes to
    /// where the cursor stood during it.
    #[test]
    fn a_long_gap_counts_five_minutes() {
        let t0 = Instant::now();
        let (a, lib) = (spot("a.rs", 3, true), spot("lib.rs", 0, false));
        let mut s = session(t0);
        s.pressed(t0 + Duration::from_secs(40), &a);
        s.moved(Some("d"), &a, &lib, None, false);
        s.pressed(t0 + Duration::from_secs(40 + 4 * 3600), &lib);
        s.moved(Some("["), &lib, &a, None, false);
        s.pressed(t0 + Duration::from_secs(40 + 4 * 3600 + 7), &a);
        assert_eq!((s.on_review.as_secs(), s.elsewhere.as_secs()), (47, 300));
        assert_eq!((s.presses, s.back), (3, 1));
    }

    /// `d` out of the review, a usage picked and a `d` further on, `[ [ [` back: one excursion
    /// of three jumps. A jump within the review, one that did not move, and one out that did not
    /// start from the review are none.
    #[test]
    fn an_excursion_counts_once_with_its_jumps() {
        let mut s = session(Instant::now());
        let mut from = spot("a.rs", 1, true);
        for (to, action) in [
            (spot("a.rs", 9, true), "d"),
            (spot("b.rs", 4, true), "Picker: Enter"),
            (spot("lib.rs", 2, false), "d"),
            (spot("lib.rs", 2, false), "u"),
            (spot("util.rs", 7, false), "Picker: Enter"),
            (spot("util.rs", 20, false), "d"),
            (spot("util.rs", 7, false), "["),
            (spot("lib.rs", 2, false), "["),
            (spot("b.rs", 4, true), "["),
            (spot("b.rs", 4, true), "]"),
            (spot("lib.rs", 2, false), "]"),
            (spot("other.rs", 1, false), "d"),
        ] {
            s.moved(Some(action), &from, &to, None, false);
            from = to;
        }
        assert_eq!((s.excursions, s.jumps, s.back), (1, 3, 3));
    }

    /// A stop on a hunk its file no longer has, or in a file that left the review, does not
    /// count: the hunks stopped on are never more than the hunks.
    #[test]
    fn stops_never_outnumber_hunks() {
        let mut s = session(Instant::now());
        let (from, to) = (spot("x.rs", 0, true), spot("a.rs", 1, true));
        for (file, i) in [("a.rs", 1), ("a.rs", 3), ("gone.rs", 1)] {
            s.moved(Some("c"), &from, &to, Some((PathBuf::from(file), i)), false);
        }
        let review = git::Review {
            branch: "feat".into(),
            base: "main".into(),
            merge_base: String::new(),
            files: Vec::new(),
            note: None,
        };
        let hunks = HashMap::from([(PathBuf::from("a.rs"), 2)]);
        let columns = s.columns(&review, &hunks, 0);
        assert_eq!(columns.split('\t').nth(9), Some("1"), "{columns}");
    }

    /// The first session of a branch is round 1, the next one round 2; another branch, or the
    /// same branch in another repository, counts its own. A session older than 30 days is
    /// dropped on the next write and no longer counts.
    #[test]
    fn rounds_count_per_branch_and_old_sessions_go() {
        let file = scratch("rounds");
        let day = |d| stats::day(d).unwrap();
        let cols = "2\t1\t3\t1\t40\t0\t0\t0\t0\t1\t2\t1";
        add(&file, day("2026-08-01"), "merl", "feat/a", cols).unwrap();
        add(&file, day("2026-09-20"), "merl", "feat/a", cols).unwrap();
        add(&file, day("2026-09-20"), "merl", "feat/b", cols).unwrap();
        add(&file, day("2026-09-21"), "other", "feat/a", cols).unwrap();
        std::fs::write(
            &file,
            std::fs::read_to_string(&file).unwrap() + "not a session\n",
        )
        .unwrap();
        add(&file, day("2026-09-26"), "merl", "feat/a", cols).unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            format!(
                "{HEAD}\n\
                 2026-09-20\tmerl\tfeat/a\t1\t{cols}\n\
                 2026-09-20\tmerl\tfeat/b\t1\t{cols}\n\
                 2026-09-21\tother\tfeat/a\t1\t{cols}\n\
                 2026-09-26\tmerl\tfeat/a\t2\t{cols}\n"
            )
        );
        // A file that cannot be read is not written over.
        std::fs::write(&file, b"\xff\n").unwrap();
        assert!(add(&file, day("2026-09-26"), "merl", "feat/a", cols).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"\xff\n");
        let _ = std::fs::remove_dir_all(file.ancestors().nth(3).unwrap());
    }

    /// `merl --reviews`: the last 30 days newest first, lines as added and deleted together,
    /// active as the time on the review plus elsewhere, then the medians by round.
    #[test]
    fn the_report_lists_sessions_newest_first_with_medians() {
        let text = format!(
            "{HEAD}\n\
             2026-08-01\tmerl\told\t1\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\t0\n\
             2026-09-24\tmerl\tfix/login-rate\t1\t7\t19\t150\t62\t910\t532\t11\t20\t14\t19\t7\t1\n\
             2026-09-25\tmerl\tfeat/paging\t1\t2\t2\t6\t2\t230\t140\t7\t9\t9\t2\t2\t1\n\
             2026-09-26\tmerl\tfeat/paging\t2\t2\t1\t3\t1\t35\t5\t0\t0\t0\t1\t2\t1\n\
             2026-09-26\tbroken\n"
        );
        let today = stats::day("2026-09-26").unwrap();
        assert_eq!(
            table(&text, today),
            "\
date        branch          round  files  hunks  lines  active  on hunks  excursions
2026-09-26  feat/paging         2      2      1      4    0:40      0:35           0
2026-09-25  feat/paging         1      2      2      8    6:10      3:50           7
2026-09-24  fix/login-rate      1      7     19    212   24:02     15:10          11

30 days: 3 sessions · median first round 15:06 · later rounds 0:40
"
        );
        assert_eq!(table("", today), "30 days: 0 sessions\n");
        assert_eq!(table(&text, today + 40), "30 days: 0 sessions\n");
    }
}
