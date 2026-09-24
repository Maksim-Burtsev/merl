//! `merl --drill [N]` (#209): N tasks of [`POOL`], each asking for what to do and never naming
//! the key, the keys left unpressed in real work asked most often. A task is checked by its
//! effect as the tutor checks it; a hit needs its action routed as well.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use super::{POOL, fresh, set_up};
use crate::app::{App, Mode, plural};
use crate::stats;

// Named constants, to recheck after two weeks of real data (#193).
/// In use in real work: at least this many presses in the last [`stats::WINDOW`] days.
const IN_USE: u64 = 3;
/// The work factor of a key missed in real work more often than pressed, and of one unused.
const MISSED_IN_WORK: u32 = 5;
const UNUSED: u32 = 4;
/// The key's last attempts the drill factor reads.
const LAST: usize = 3;
/// The drill factor of a miss among them, and of their median over [`SLOW`] × the typical hit.
const MISSED: u32 = 3;
const SLOWER: u32 = 2;
/// Slow: over this many times the median of the last [`HITS`] hits, across all keys.
const SLOW: f64 = 1.5;
const HITS: usize = 100;
/// A key is not asked again until this many other tasks have passed.
const GAP: usize = 2;
/// A missed task comes back, its key not named, this many tasks later.
const AGAIN: usize = 3;

/// An attempt: the action a task trains and its time in ms, `None` for a miss.
type Attempt = (&'static str, Option<u32>);

pub struct Drill {
    /// Tasks in the session.
    n: usize,
    /// This session's answers: the task, as an index into [`POOL`], and its time.
    answers: Vec<(usize, Option<u32>)>,
    /// Every attempt, the log's and then this session's, the oldest first.
    attempts: Vec<Attempt>,
    /// Each action's presses and misses in real work, the last [`stats::WINDOW`] days.
    work: HashMap<&'static str, (u64, u64)>,
    /// `drill.tsv`, a line appended after each answer; `None` writes nothing.
    log: Option<PathBuf>,
    rng: u64,
    started: Instant,
    /// The task on screen; `None` once the session is over.
    cur: Option<Cur>,
}

struct Cur {
    task: usize,
    /// The redo right after a miss: its text names the key, and it is no attempt.
    named: bool,
    /// Once its start is set up.
    since: Instant,
    /// The task's action was routed.
    hit: bool,
    /// `?` was opened.
    looked: bool,
}

impl Drill {
    /// A session of `n` tasks, weighed by `keys.tsv` and the attempts in `log`.
    pub fn new(n: usize, keys: Option<&Path>, log: Option<PathBuf>, today: i64) -> Result<Drill> {
        let work = match keys {
            Some(keys) => stats::month(keys, today)?,
            None => HashMap::new(),
        };
        let attempts = match log
            .as_deref()
            .map(|log| (log, std::fs::read_to_string(log)))
        {
            Some((_, Ok(text))) => parse(&text),
            Some((_, Err(e))) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Some((log, Err(e))) => return Err(e).with_context(|| format!("{}", log.display())),
            None => Vec::new(),
        };
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Ok(Drill {
            n,
            answers: Vec::new(),
            attempts,
            work,
            log,
            rng: seed,
            started: Instant::now(),
            cur: None,
        })
    }

    /// Work factor × drill factor; never below 1, so no key drops out of the pool.
    fn weight(&self, key: &str) -> u32 {
        let (pressed, missed) = self.work.get(key).copied().unwrap_or_default();
        let work = if missed > pressed {
            MISSED_IN_WORK
        } else if pressed < IN_USE {
            UNUSED
        } else {
            1
        };
        let last: Vec<Option<u32>> = (self.attempts.iter().rev())
            .filter(|a| a.0 == key)
            .take(LAST)
            .map(|a| a.1)
            .collect();
        let drill = if last.contains(&None) {
            MISSED
        } else if median(last.into_iter().flatten().collect()).is_some_and(|ms| self.slow(ms)) {
            SLOWER
        } else {
            1
        };
        work * drill
    }

    /// Over [`SLOW`] × the median of the last [`HITS`] hits, as many as there are.
    fn slow(&self, ms: u32) -> bool {
        let hits = self.attempts.iter().rev().filter_map(|a| a.1).take(HITS);
        median(hits.collect()).is_some_and(|typical| f64::from(ms) > SLOW * f64::from(typical))
    }

    /// The next slot's task: the one missed [`AGAIN`] slots before, else the first never
    /// attempted in `KEYS` order, else one at random by weight among those [`GAP`] slots clear;
    /// `None` once all N are answered.
    fn pick(&mut self) -> Option<usize> {
        let slot = self.answers.len();
        if slot >= self.n {
            return None;
        }
        if let Some(&(task, None)) = slot.checked_sub(AGAIN).map(|s| &self.answers[s]) {
            return Some(task);
        }
        let unseen = stats::ACTIONS
            .iter()
            .filter(|a| !self.attempts.iter().any(|t| t.0 == a.name))
            .find_map(|a| POOL.iter().position(|t| t.key == a.name));
        if unseen.is_some() {
            return unseen;
        }
        let recent: Vec<usize> = self.answers[slot.saturating_sub(GAP)..]
            .iter()
            .map(|a| a.0)
            .collect();
        let weights: Vec<u64> = (0..POOL.len())
            .map(|t| match recent.contains(&t) {
                true => 0,
                false => u64::from(self.weight(POOL[t].key)),
            })
            .collect();
        let mut r = self.random() % weights.iter().sum::<u64>();
        weights.iter().position(|&w| {
            if r < w {
                return true;
            }
            r -= w;
            false
        })
    }

    /// splitmix64.
    fn random(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Records an attempt and appends it to the log at once: parallel merls need no merge, and
    /// `q` or a crash keeps the answers given.
    fn answer(&mut self, task: usize, ms: Option<u32>) -> Result<()> {
        let key = POOL[task].key;
        self.answers.push((task, ms));
        self.attempts.push((key, ms));
        let Some(log) = &self.log else {
            return Ok(());
        };
        let ctx = || format!("{}", log.display());
        if let Some(dir) = log.parent() {
            std::fs::create_dir_all(dir).with_context(ctx)?;
        }
        let ms = ms.map_or("miss".to_string(), |ms| ms.to_string());
        let line = format!("{}\t{key}\t{ms}\n", stats::date(stats::today()));
        let mut file = (std::fs::OpenOptions::new().create(true).append(true))
            .open(log)
            .with_context(ctx)?;
        file.write_all(line.as_bytes()).with_context(ctx)
    }

    /// The last task is answered: merl quits.
    pub fn over(&self) -> bool {
        self.cur.is_none()
    }

    /// The panel's title bar and text: the redo keeps the slot of the miss, and names the key.
    pub fn panel(&self) -> (String, &'static str) {
        let Some(cur) = &self.cur else {
            return (String::new(), "");
        };
        let slot = self.answers.len() + usize::from(!cur.named);
        let task = &POOL[cur.task];
        let text = if cur.named { task.tutor } else { task.drill };
        (format!(" Drill {slot}/{}", self.n), text)
    }

    /// What merl prints on exit: the tasks answered, and this session's weak keys, its misses
    /// first, then its slow hits, the slowest first.
    pub fn summary(&self) -> String {
        let secs = self.started.elapsed().as_secs();
        let n = self.answers.len();
        let mut out = format!(
            "drill: {n} task{} in {}:{:02}\n",
            plural(n),
            secs / 60,
            secs % 60
        );
        // Misses and the slowest slow hit, by task.
        let mut weak: Vec<(usize, u32, usize)> = Vec::new();
        for &(task, ms) in &self.answers {
            let i = weak.iter().position(|w| w.2 == task).unwrap_or_else(|| {
                weak.push((0, 0, task));
                weak.len() - 1
            });
            match ms {
                None => weak[i].0 += 1,
                Some(ms) if self.slow(ms) => weak[i].1 = weak[i].1.max(ms),
                Some(_) => {}
            }
        }
        weak.retain(|w| w.0 > 0 || w.1 > 0);
        weak.sort_by_key(|w| std::cmp::Reverse((w.0, w.1)));
        if weak.is_empty() {
            out.push_str("no weak keys\n");
        }
        // As wide as `Alt+Shift+Right` and a space, or the longest key and a space.
        let w = weak
            .iter()
            .map(|w| POOL[w.2].key.len() + 1)
            .fold(16, usize::max);
        for (misses, ms, task) in weak {
            let verdict = match misses {
                0 => format!("{:.1} s", f64::from(ms) / 1000.0),
                n => format!("{n} miss{}", if n == 1 { "" } else { "es" }),
            };
            let (key, title) = (POOL[task].key, POOL[task].title);
            _ = writeln!(out, "  {key:<w$} {verdict:<10} {title}");
        }
        out
    }
}

/// `YYYY-MM-DD<TAB>action<TAB>ms` or `miss`; a line that is neither, or names an action no
/// task trains any more, is dropped. The date is not read: attempts do not age.
fn parse(text: &str) -> Vec<Attempt> {
    text.lines()
        .filter_map(|line| {
            let mut cols = line.split('\t').skip(1);
            let (key, ms) = (cols.next()?, cols.next()?);
            let key = POOL.iter().find(|t| t.key == key)?.key;
            let ms = match ms {
                "miss" => None,
                ms => Some(ms.parse().ok()?),
            };
            Some((key, ms))
        })
        .collect()
}

fn median(mut v: Vec<u32>) -> Option<u32> {
    v.sort_unstable();
    let mid = v.len() / 2;
    match v.len() {
        0 => None,
        n if n % 2 == 1 => Some(v[mid]),
        _ => Some(((u64::from(v[mid - 1]) + u64::from(v[mid])) / 2) as u32),
    }
}

fn drill(app: &mut App) -> Option<&mut Drill> {
    app.tutor.as_mut()?.drill.as_mut()
}

/// Called after every key with the action it was routed to, and after a jump a search made:
/// once the task is done, logs it and sets the next one up, or the same one with its key
/// named after a miss.
pub fn check(app: &mut App, action: Option<&str>) {
    let help = app.mode == Mode::Help;
    let Some(cur) = drill(app).and_then(|d| d.cur.as_mut()) else {
        return;
    };
    cur.hit |= action == Some(POOL[cur.task].key);
    cur.looked |= action == Some("?") && help;
    let (task, named, hit, since) = (cur.task, cur.named, cur.hit && !cur.looked, cur.since);
    if !(POOL[task].done)(app) {
        return;
    }
    let ms = since.elapsed().as_millis() as u32;
    let Some(d) = drill(app) else {
        return;
    };
    let (said, logged) = match (named, hit) {
        (true, _) => ("\u{2713}".to_string(), Ok(())),
        (false, true) => (
            format!("\u{2713} {:.1} s", f64::from(ms) / 1000.0),
            d.answer(task, Some(ms)),
        ),
        (false, false) => ("miss".to_string(), d.answer(task, None)),
    };
    let set = match named || hit {
        true => next(app),
        false => run(app, task, true),
    };
    app.message = match logged.and(set) {
        Ok(()) => said,
        Err(e) => format!("{said}  {e:#}"),
    };
}

/// Sets the next slot's task up, or ends the session.
pub fn next(app: &mut App) -> Result<()> {
    let Some(d) = drill(app) else {
        return Ok(());
    };
    match d.pick() {
        Some(task) => run(app, task, false),
        None => {
            d.cur = None;
            Ok(())
        }
    }
}

/// `task` from its start on a fresh App; the clock starts once it is set up.
fn run(app: &mut App, task: usize, named: bool) -> Result<()> {
    let set = fresh(app).map(|()| set_up(app, &POOL[task]));
    if let Some(d) = drill(app) {
        d.cur = Some(Cur {
            task,
            named,
            since: Instant::now(),
            hit: false,
            looked: false,
        });
    }
    set
}

#[cfg(test)]
mod tests {
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::stats;
    use crate::tutor::press;
    use crate::tutor::tests::{app, dir};

    /// The task that trains `key`.
    fn task(key: &str) -> usize {
        POOL.iter().position(|t| t.key == key).unwrap()
    }

    /// A drill of `n` tasks that reads and writes no file.
    fn drill(n: usize) -> Drill {
        let mut d = Drill::new(n, None, None, stats::today()).unwrap();
        d.rng = 7;
        d
    }

    /// Asks `d` to its end, missing the slots in `missed` and taking a second over each hit.
    fn session(d: &mut Drill, missed: &[usize]) -> Vec<&'static str> {
        let mut asked = Vec::new();
        while let Some(t) = d.pick() {
            let ms = (!missed.contains(&asked.len())).then_some(1000);
            asked.push(POOL[t].key);
            d.answer(t, ms).unwrap();
        }
        asked
    }

    /// The pool in `KEYS` order, the order the drill asks unattempted keys in.
    fn in_keys_order() -> Vec<&'static str> {
        stats::ACTIONS
            .iter()
            .filter_map(|a| POOL.iter().find(|t| t.key == a.name))
            .map(|t| t.key)
            .collect()
    }

    /// One key per row of the table, from a `keys.tsv` and a `drill.tsv` on disk.
    #[test]
    fn a_weight_is_the_work_factor_times_the_drill_factor() {
        let dir = dir("weights");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let today = stats::today();
        let ago = |days| stats::date(today - days);
        // Work: `d` missed more often than pressed; `u` pressed twice, and 50 times on the first
        // day out of the window; `v` never; the rest three times, on the last day in it.
        let mut keys = format!(
            "{}\td\t2\t5\n{}\tu\t2\t0\n{}\tu\t50\t0\n",
            ago(1),
            ago(1),
            ago(30)
        );
        for k in ["o", "[", "]", "}", "{"] {
            keys += &format!("{}\t{k}\t3\t1\n", ago(29));
        }
        std::fs::write(dir.join("keys.tsv"), keys).unwrap();
        // Drill: a hit takes a second, going by the last 100 hits; the 200 slow ones before them
        // are out of the median.
        let mut log = String::new();
        for ms in [5000; 200].into_iter().chain([1000; 100]) {
            log += &format!("{}\tn\t{ms}\n", ago(9));
        }
        for (key, times) in [
            ("d", "900 900 900"),
            ("u", "900 900 900"),
            ("o", "900 900 900"),
            ("[", "900 miss 900"),
            ("]", "1600 1600 1600"),
            ("}", "1500 1500 1500"),
            ("{", "miss 900 900 900"),
            ("v", "1600 1600 1600"),
        ] {
            for ms in times.split(' ') {
                log += &format!("{}\t{key}\t{ms}\n", ago(2));
            }
        }
        std::fs::write(dir.join("drill.tsv"), log).unwrap();
        let d = Drill::new(
            20,
            Some(&dir.join("keys.tsv")),
            Some(dir.join("drill.tsv")),
            today,
        )
        .unwrap();
        for (key, weight, why) in [
            ("d", 5, "missed in work more often than pressed, fast"),
            (
                "u",
                4,
                "unused in work: under 3 presses in the window, fast",
            ),
            ("o", 1, "in use, fast: the floor"),
            ("[", 3, "in use, a miss among the last 3 attempts"),
            (
                "]",
                2,
                "in use, slow: over 1.5 x the median of the last 100 hits",
            ),
            ("}", 1, "in use, at 1.5 x the median: not over it"),
            ("{", 1, "in use, the miss 4 attempts back"),
            ("v", 8, "unused and slow: a product, not the maximum"),
        ] {
            assert_eq!(d.weight(key), weight, "{key}: {why}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn keys_never_attempted_come_first_in_keys_order() {
        let mut d = drill(20);
        let fresh = ["Ctrl+N", "Tab", "Picker: PgDn"];
        d.attempts = in_keys_order()
            .into_iter()
            .filter(|k| !fresh.contains(k))
            .map(|k| (k, Some(1000)))
            .collect();
        let asked = session(&mut d, &[]);
        assert_eq!(asked[..3], fresh);
        assert_eq!(asked.len(), 20);
        // With nothing attempted, the first session is the start of the tour.
        assert_eq!(session(&mut drill(20), &[]), in_keys_order()[..20]);
    }

    /// Weighted random over a long session with a fixed seed: the heavy key is asked more often
    /// than its share, and no key twice within two tasks of itself.
    #[test]
    fn a_key_waits_two_other_tasks_before_it_is_asked_again() {
        let mut d = drill(400);
        d.attempts = in_keys_order()
            .into_iter()
            .map(|k| (k, Some(1000)))
            .collect();
        d.work = POOL.iter().map(|t| (t.key, (3, 0))).collect();
        d.work.insert("s", (0, 1));
        let asked = session(&mut d, &[]);
        assert_eq!(asked.len(), 400);
        for (i, key) in asked.iter().enumerate() {
            let near = &asked[i + 1..(i + 3).min(asked.len())];
            assert!(!near.contains(key), "{key} at {i} and again at {near:?}");
        }
        let s = asked.iter().filter(|k| **k == "s").count();
        assert!(s > 2 * 400 / POOL.len(), "`s` asked {s} times");
    }

    /// Slot 0 is missed and asked again at slot 3, missed again and asked at slot 6; slot 17's
    /// miss would come back at slot 20, past the end of the session.
    #[test]
    fn a_miss_comes_back_three_tasks_later_within_the_session() {
        let asked = session(&mut drill(20), &[0, 3, 17]);
        let k = in_keys_order();
        let mut want = vec![k[0], k[1], k[2], k[0], k[3], k[4], k[0]];
        want.extend(&k[5..18]);
        assert_eq!(asked, want);
    }

    /// The panel's title and text.
    fn on_screen(a: &App) -> (String, &'static str) {
        a.tutor.as_ref().unwrap().drill.as_ref().unwrap().panel()
    }

    /// A drill of `n` tasks over the copy `name` of the sample project, logging to a file of its
    /// own.
    fn drill_app(name: &str, n: usize) -> (App, PathBuf) {
        let mut a = app(name, (80, 24));
        let log = dir(name).with_extension("tsv");
        let _ = std::fs::remove_file(&log);
        let d = Drill::new(n, None, Some(log.clone()), stats::today()).unwrap();
        a.tutor.as_mut().unwrap().drill = Some(d);
        (a, log)
    }

    /// The log's lines without their dates.
    fn logged(log: &Path) -> Vec<String> {
        let text = std::fs::read_to_string(log).unwrap_or_default();
        text.lines()
            .map(|l| l.split_once('\t').unwrap().1.to_string())
            .collect()
    }

    fn clean_up(name: &str, log: &Path) {
        let _ = std::fs::remove_dir_all(dir(name));
        let _ = std::fs::remove_file(log);
    }

    /// Backspace x11 in place of Alt+Backspace: a miss, then the same task again with its key
    /// named, which is no attempt and not logged; the counter moves on after it.
    #[test]
    fn a_task_done_another_way_is_a_miss_and_its_redo_is_not_logged() {
        let (mut a, log) = drill_app("drill-miss", 2);
        let t = task("Edit: Alt+Backspace");
        run(&mut a, t, false).unwrap();
        assert_eq!(on_screen(&a), (" Drill 1/2".to_string(), POOL[t].drill));
        press(&mut a, &"<BS>".repeat(11));
        assert_eq!(a.message, "miss");
        assert_eq!(logged(&log), ["Edit: Alt+Backspace\tmiss"]);
        assert_eq!(on_screen(&a), (" Drill 1/2".to_string(), POOL[t].tutor));
        assert!(!(POOL[t].done)(&a), "the redo starts from the task's start");
        press(&mut a, POOL[t].answer);
        assert_eq!(a.message, "\u{2713}");
        assert_eq!(logged(&log).len(), 1, "the redo is logged");
        let o = task("o");
        assert_eq!(on_screen(&a), (" Drill 2/2".to_string(), POOL[o].drill));
        press(&mut a, POOL[o].answer);
        assert!(
            a.message.starts_with("\u{2713} ") && a.message.ends_with(" s"),
            "{}",
            a.message
        );
        assert!(logged(&log)[1].starts_with("o\t"), "{:?}", logged(&log));
        assert!(a.tutor.as_ref().unwrap().drill.as_ref().unwrap().over());
        // Nothing of it reaches the key stats.
        assert!(a.pressed.is_empty() && a.missed.is_empty());
        clean_up("drill-miss", &log);
    }

    /// `?` opened during a task makes it a miss, whatever keys did it; Shift+F12 on the `u`
    /// task is `u`, a hit.
    #[test]
    fn the_help_opened_is_a_miss_and_an_alias_is_a_hit() {
        let (mut a, log) = drill_app("drill-help", 20);
        run(&mut a, task("d"), false).unwrap();
        press(&mut a, "?<Esc>d");
        assert_eq!(a.message, "miss");
        press(&mut a, "d");
        run(&mut a, task("u"), false).unwrap();
        a.key(KeyEvent::new(KeyCode::F(12), KeyModifiers::SHIFT));
        press(&mut a, "<Down><Enter>");
        assert!(a.message.starts_with("\u{2713} "), "{}", a.message);
        let lines = logged(&log);
        assert_eq!(lines[0], "d\tmiss");
        assert!(
            lines[1]
                .strip_prefix("u\t")
                .is_some_and(|ms| ms.parse::<u32>().is_ok())
        );
        clean_up("drill-help", &log);
    }

    /// This session's misses, then its hits over 1.5 x the median, the slowest first.
    #[test]
    fn the_summary_lists_the_sessions_misses_then_its_slow_hits() {
        let mut d = drill(20);
        d.attempts = vec![("n", Some(1000)); 50];
        for (key, ms) in [
            ("Picker: PgDn", None),
            ("o", Some(900)),
            ("d", Some(1600)),
            ("Alt+Shift+Right", Some(3000)),
            ("Picker: PgDn", None),
            ("Picker: PgDn", Some(1000)),
        ] {
            d.answer(task(key), ms).unwrap();
        }
        assert_eq!(
            d.summary(),
            "drill: 6 tasks in 0:00\n\
             \x20 Picker: PgDn     2 misses   A page down a list\n\
             \x20 Alt+Shift+Right  3.0 s      Select words\n\
             \x20 d                1.6 s      Go to definition\n"
        );
        let mut d = drill(20);
        d.answer(task("o"), Some(1000)).unwrap();
        assert_eq!(d.summary(), "drill: 1 task in 0:00\nno weak keys\n");
        // A key longer than the column still has two spaces after it.
        d.answer(task("Edit: Alt+Backspace"), None).unwrap();
        assert!(
            d.summary()
                .contains("\n  Edit: Alt+Backspace  1 miss     Delete a word\n"),
            "{}",
            d.summary()
        );
    }
}
