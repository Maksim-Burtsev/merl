use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
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

/// The open file, the cursor's line in it (a deleted one included), and whether the file is one
/// of the review's.
pub type Spot = (Option<PathBuf>, git::TextLine, bool);

/// A stop of the review walk, as the status bar's `hunk i/n` numbers it: the file relative to
/// the root and the hunk's place in it, from 1; the top of a deleted file is its one stop.
pub type Stop = (PathBuf, usize);

/// The stops `c` can make in each file of the review as it opened, counted on a thread started
/// then, so that neither the first frame nor quitting waits for a `git diff` per file; `None`
/// when one failed.
pub type Counting = JoinHandle<Option<HashMap<PathBuf, usize>>>;

/// What one review session did, counted on each press.
pub struct Session {
    /// The repository and the branch the session is of, and the review's size, all taken when
    /// it opened: a `git switch` during it does not file it under another branch, nor fill the
    /// row with the other branch's review.
    pub repo: String,
    pub branch: String,
    added: usize,
    deleted: usize,
    counting: Option<Counting>,
    stops_in: Option<HashMap<PathBuf, usize>>,
    /// The last press, or the start of the review.
    last: Instant,
    /// Left the review by a jump and not back on one of its files yet.
    out: bool,
    /// The presses but the one that quits.
    presses: u64,
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
    /// The review `review` of `branch` in `repo`, opened at `at` on the stop `stop` when it
    /// opened on one.
    pub fn new(
        at: Instant,
        repo: String,
        branch: String,
        review: &git::Review,
        counting: Counting,
        stop: Option<Stop>,
    ) -> Self {
        let lines = |f: fn(&git::ReviewFile) -> usize| review.files.iter().map(f).sum();
        Self {
            repo,
            branch,
            added: lines(|f| f.added),
            deleted: lines(|f| f.deleted),
            counting: Some(counting),
            stops_in: None,
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
    /// there, [`IDLE`] at most. The key that quits (`quit`) is no press of its own, so a session
    /// of `q` alone is none; the reading before it still counts.
    pub fn pressed(&mut self, at: Instant, from: &Spot, quit: bool) {
        let gap = at.saturating_duration_since(self.last).min(IDLE);
        let total = match from.2 {
            true => &mut self.on_review,
            false => &mut self.elsewhere,
        };
        *total += gap;
        self.last = at;
        self.presses += u64::from(!quit);
    }

    /// `action` (a `KEYS` action) took the cursor from `from` to `to`; `stop`: the review walk
    /// left it on that stop; `end`: it was `c` saying `last hunk of the review`.
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

    /// The session's columns from `files` on, as [`HEAD`] names them, for the review as it
    /// opened: a stop on a hunk it did not have, or a file marked viewed that it did not list,
    /// does not count, so neither is ever more than the hunks or the files. `None` for a
    /// session without a press, or when counting the stops failed.
    pub fn columns<'a>(&mut self, viewed: impl Iterator<Item = &'a PathBuf>) -> Option<String> {
        if self.presses == 0 {
            return None;
        }
        if let Some(counting) = self.counting.take() {
            self.stops_in = counting.join().ok().flatten();
        }
        let stops_in = self.stops_in.as_ref()?;
        let stops = (self.stops.iter())
            .filter(|(path, i)| stops_in.get(path).is_some_and(|n| i <= n))
            .count();
        let columns = [
            stops_in.len(),
            stops_in.values().sum(),
            self.added,
            self.deleted,
            self.on_review.as_secs() as usize,
            self.elsewhere.as_secs() as usize,
            self.excursions as usize,
            self.jumps as usize,
            self.back as usize,
            stops,
            viewed.filter(|path| stops_in.contains_key(*path)).count(),
            usize::from(self.last_hunk),
        ];
        Some(columns.map(|n| n.to_string()).join("\t"))
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
        (Some(PathBuf::from(file)), git::TextLine::File(line), on)
    }

    fn session_over(at: Instant, stops_in: &[(&str, usize)]) -> Session {
        let review = git::Review {
            branch: "feat".into(),
            base: "main".into(),
            merge_base: String::new(),
            files: Vec::new(),
            deleted: Default::default(),
            added: Default::default(),
            note: None,
        };
        let stops_in: HashMap<PathBuf, usize> = stops_in
            .iter()
            .map(|&(f, n)| (PathBuf::from(f), n))
            .collect();
        let counting = std::thread::spawn(move || Some(stops_in));
        Session::new(at, "merl".into(), "feat".into(), &review, counting, None)
    }

    fn session(at: Instant) -> Session {
        session_over(at, &[])
    }

    /// A gap up to five minutes counts in full, a longer one as five minutes, and it goes to
    /// where the cursor stood during it.
    #[test]
    fn a_long_gap_counts_five_minutes() {
        let t0 = Instant::now();
        let (a, lib) = (spot("a.rs", 3, true), spot("lib.rs", 0, false));
        let mut s = session(t0);
        s.pressed(t0 + Duration::from_secs(40), &a, false);
        s.moved(Some("d"), &a, &lib, None, false);
        s.pressed(t0 + Duration::from_secs(40 + 4 * 3600), &lib, false);
        s.moved(Some("["), &lib, &a, None, false);
        s.pressed(t0 + Duration::from_secs(40 + 4 * 3600 + 7), &a, false);
        assert_eq!((s.on_review.as_secs(), s.elsewhere.as_secs()), (47, 300));
        assert_eq!((s.presses, s.back), (3, 1));
    }

    /// The key that quits is no press: a session of `q` alone has no line, though the reading
    /// before a `q` counts in one that has.
    #[test]
    fn the_key_that_quits_is_no_press() {
        let t0 = Instant::now();
        let a = spot("a.rs", 3, true);
        let mut s = session(t0);
        s.pressed(t0 + Duration::from_secs(20), &a, true);
        assert_eq!(s.columns(std::iter::empty()), None);
        s.pressed(t0 + Duration::from_secs(30), &a, false);
        s.pressed(t0 + Duration::from_secs(45), &a, true);
        let columns = s.columns(std::iter::empty()).unwrap();
        assert_eq!(columns.split('\t').nth(4), Some("45"), "{columns}");
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

    /// A jump that lands back on a file of the review ends the excursion and is none of its
    /// jumps.
    #[test]
    fn a_jump_back_into_the_review_is_no_excursion_jump() {
        let mut s = session(Instant::now());
        let (a, lib, b) = (
            spot("a.rs", 1, true),
            spot("lib.rs", 2, false),
            spot("b.rs", 4, true),
        );
        s.moved(Some("d"), &a, &lib, None, false);
        s.moved(Some("Picker: Enter"), &lib, &b, None, false);
        assert_eq!((s.excursions, s.jumps, s.out), (1, 1, false));
    }

    /// A stop on a hunk the review did not have when it opened, or in a file it did not list,
    /// does not count, nor does a file marked viewed that it did not list: neither is ever more
    /// than the hunks or the files.
    #[test]
    fn stops_and_viewed_never_outnumber_the_review() {
        let t0 = Instant::now();
        let mut s = session_over(t0, &[("a.rs", 2), ("b.rs", 1)]);
        let (from, to) = (spot("x.rs", 0, true), spot("a.rs", 1, true));
        for (file, i) in [("a.rs", 1), ("a.rs", 3), ("gone.rs", 1)] {
            s.moved(Some("c"), &from, &to, Some((PathBuf::from(file), i)), false);
        }
        s.pressed(t0, &to, false);
        let viewed = [PathBuf::from("a.rs"), PathBuf::from("new.rs")];
        let columns = s.columns(viewed.iter()).unwrap();
        let cols: Vec<&str> = columns.split('\t').collect();
        assert_eq!(
            (cols[0], cols[1], cols[9], cols[10]),
            ("2", "3", "1", "1"),
            "files, hunks, stops, viewed: {columns}"
        );
    }

    /// A session's line goes into the file under the head's names, and `merl --reviews` reads
    /// back what went in.
    #[test]
    fn a_line_round_trips_through_the_file() {
        let file = scratch("roundtrip");
        let t0 = Instant::now();
        let row = |path: &str, added, deleted| git::ReviewFile {
            path: PathBuf::from(path),
            status: 'M',
            old: None,
            added,
            deleted,
            binary: false,
            untracked: false,
            generated: false,
        };
        let review = git::Review {
            branch: "feat/x".into(),
            base: "main".into(),
            merge_base: String::new(),
            files: vec![row("a.rs", 3, 1), row("b.rs", 2, 0)],
            deleted: Default::default(),
            added: Default::default(),
            note: None,
        };
        let stops_in = HashMap::from([(PathBuf::from("a.rs"), 2), (PathBuf::from("b.rs"), 1)]);
        let counting = std::thread::spawn(move || Some(stops_in));
        let a1 = Some((PathBuf::from("a.rs"), 1));
        let mut s = Session::new(t0, "merl".into(), "feat/x".into(), &review, counting, a1);
        let (a, b) = (spot("a.rs", 1, true), spot("b.rs", 0, true));
        let (lib, util) = (spot("lib.rs", 2, false), spot("util.rs", 5, false));
        let at = |secs| t0 + Duration::from_secs(secs);
        // 100 s on `a.rs`, `d` out; 30 s, `d` further; 20 s, `[` back; 10 s, `c` to the end.
        s.pressed(at(100), &a, false);
        s.moved(Some("d"), &a, &lib, None, false);
        s.pressed(at(130), &lib, false);
        s.moved(Some("d"), &lib, &util, None, false);
        s.pressed(at(150), &util, false);
        s.moved(Some("["), &util, &a, None, false);
        s.pressed(at(160), &a, false);
        s.moved(Some("c"), &a, &b, Some((PathBuf::from("b.rs"), 1)), true);
        let columns = s.columns([PathBuf::from("a.rs")].iter()).unwrap();
        let today = stats::day("2026-09-26").unwrap();
        add(&file, today, "merl", "feat/x", &columns).unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let named: Vec<(&str, &str)> = HEAD.split('\t').zip(lines[1].split('\t')).collect();
        assert_eq!(
            named,
            [
                ("date", "2026-09-26"),
                ("repo", "merl"),
                ("branch", "feat/x"),
                ("round", "1"),
                ("files", "2"),
                ("hunks", "3"),
                ("added", "5"),
                ("deleted", "1"),
                ("on_review_s", "110"),
                ("elsewhere_s", "50"),
                ("excursions", "1"),
                ("jumps", "2"),
                ("back", "1"),
                ("stops", "2"),
                ("viewed", "1"),
                ("last_hunk", "1"),
            ]
        );
        assert_eq!(lines[1].split('\t').count(), HEAD.split('\t').count());
        let out = report(&file, today).unwrap();
        let cells: Vec<&str> = out.lines().nth(1).unwrap().split_whitespace().collect();
        assert_eq!(
            cells,
            [
                "2026-09-26",
                "feat/x",
                "1",
                "2",
                "3",
                "6",
                "2:40",
                "1:50",
                "1"
            ]
        );
        let _ = std::fs::remove_dir_all(file.ancestors().nth(3).unwrap());
    }

    /// The median of an odd count is the middle one once sorted, of an even count the mean of
    /// the middle two.
    #[test]
    fn the_median_sorts_first() {
        assert_eq!(median(vec![900, 100, 500]), Some(500));
        assert_eq!(median(vec![900, 100, 500, 300]), Some(400));
        assert_eq!(median(Vec::new()), None);
    }

    /// The first session of a branch is round 1, the next one round 2; another branch, or the
    /// same branch in another repository, counts its own. A session of the last 30 days, today
    /// included, is kept; an older one is dropped on the next write and no longer counts.
    #[test]
    fn rounds_count_per_branch_and_old_sessions_go() {
        let file = scratch("rounds");
        let day = |d| stats::day(d).unwrap();
        let cols = "2\t1\t3\t1\t40\t0\t0\t0\t0\t1\t2\t1";
        add(&file, day("2026-08-01"), "merl", "feat/a", cols).unwrap();
        // 31, 30 and 29 days before the last write below.
        add(&file, day("2026-08-26"), "merl", "edge/31", cols).unwrap();
        add(&file, day("2026-08-27"), "merl", "edge/30", cols).unwrap();
        add(&file, day("2026-08-28"), "merl", "edge/29", cols).unwrap();
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
                 2026-08-28\tmerl\tedge/29\t1\t{cols}\n\
                 2026-09-20\tmerl\tfeat/a\t1\t{cols}\n\
                 2026-09-20\tmerl\tfeat/b\t1\t{cols}\n\
                 2026-09-21\tother\tfeat/a\t1\t{cols}\n\
                 2026-09-26\tmerl\tfeat/a\t2\t{cols}\n"
            )
        );
        std::fs::write(&file, b"\xff\n").unwrap();
        assert!(add(&file, day("2026-09-26"), "merl", "feat/a", cols).is_err());
        assert_eq!(
            std::fs::read(&file).unwrap(),
            b"\xff\n",
            "a file that cannot be read is not written over"
        );
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
