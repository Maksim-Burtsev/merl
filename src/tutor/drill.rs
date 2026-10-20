use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

use super::{POOL, REVIEW_LESSONS, fresh, set_up};
use crate::app::{App, Mode, plural};
use crate::stats;

const MIN_PRESSES_IN_USE_IN_WINDOW: u64 = 3;
const WORK_FACTOR_MISSED_MORE_THAN_PRESSED: u32 = 5;
const WORK_FACTOR_UNUSED: u32 = 4;
const LAST_ATTEMPTS_READ: usize = 3;
const DRILL_FACTOR_MISS_AMONG_LAST: u32 = 3;
const DRILL_FACTOR_SLOW_MEDIAN_OF_LAST: u32 = 2;
const SLOW_TIMES_TYPICAL_HIT: f64 = 1.5;
const TYPICAL_HIT_OF_LAST_HITS: usize = 100;
const OTHER_TASKS_BEFORE_KEY_AGAIN: usize = 2;
const MISSED_TASK_BACK_UNNAMED_AFTER: usize = 3;
const SUMMARY_KEY_COLUMN: usize = "Alt+Shift+Right ".len();

type Attempt = (&'static str, Option<u32>);

pub struct Drill {
    n: usize,
    answers: Vec<(usize, Option<u32>)>,
    attempts: Vec<Attempt>,
    work: HashMap<&'static str, (u64, u64)>,
    log: Option<PathBuf>,
    rng: u64,
    started: Instant,
    cur: Option<Cur>,
    no_review: bool,
}

struct Cur {
    task: usize,
    named: bool,
    since: Instant,
    hit: bool,
    looked: bool,
}

impl Drill {
    pub fn new(n: usize, keys: Option<&Path>, log: Option<PathBuf>, today: i64) -> Result<Self> {
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
            .map_or(0, |d| u64::try_from(d.as_nanos()).unwrap_or(u64::MAX));
        Ok(Self {
            n,
            answers: Vec::new(),
            attempts,
            work,
            log,
            rng: seed,
            started: Instant::now(),
            cur: None,
            no_review: false,
        })
    }

    fn weight(&self, key: &str) -> u32 {
        let (pressed, missed) = self.work.get(key).copied().unwrap_or_default();
        let work = if missed > pressed {
            WORK_FACTOR_MISSED_MORE_THAN_PRESSED
        } else if pressed < MIN_PRESSES_IN_USE_IN_WINDOW {
            WORK_FACTOR_UNUSED
        } else {
            1
        };
        let last: Vec<Option<u32>> = (self.attempts.iter().rev())
            .filter(|a| a.0 == key)
            .take(LAST_ATTEMPTS_READ)
            .map(|a| a.1)
            .collect();
        let drill = if last.contains(&None) {
            DRILL_FACTOR_MISS_AMONG_LAST
        } else if median(last.into_iter().flatten().collect()).is_some_and(|ms| self.slow(ms)) {
            DRILL_FACTOR_SLOW_MEDIAN_OF_LAST
        } else {
            1
        };
        work * drill
    }

    fn slow(&self, ms: u32) -> bool {
        let hits = self
            .attempts
            .iter()
            .rev()
            .filter_map(|a| a.1)
            .take(TYPICAL_HIT_OF_LAST_HITS);
        median(hits.collect())
            .is_some_and(|typical| f64::from(ms) > SLOW_TIMES_TYPICAL_HIT * f64::from(typical))
    }

    fn can_set_up(&self, task: usize) -> bool {
        !(self.no_review && REVIEW_LESSONS.contains(&POOL[task].key))
    }

    fn pick(&mut self) -> Option<usize> {
        let slot = self.answers.len();
        if slot >= self.n {
            return None;
        }
        if let Some(&(task, None)) = slot
            .checked_sub(MISSED_TASK_BACK_UNNAMED_AFTER)
            .map(|s| &self.answers[s])
            .filter(|&&(t, _)| self.can_set_up(t))
        {
            return Some(task);
        }
        let unseen = stats::ACTIONS
            .iter()
            .filter(|a| !self.attempts.iter().any(|t| t.0 == a.name))
            .filter_map(|a| POOL.iter().position(|t| t.key == a.name))
            .find(|&t| self.can_set_up(t));
        if unseen.is_some() {
            return unseen;
        }
        let recent: Vec<usize> = self.answers[slot.saturating_sub(OTHER_TASKS_BEFORE_KEY_AGAIN)..]
            .iter()
            .map(|a| a.0)
            .collect();
        let weights: Vec<u64> = (0..POOL.len())
            .map(|t| {
                if recent.contains(&t) || !self.can_set_up(t) {
                    0
                } else {
                    u64::from(self.weight(POOL[t].key))
                }
            })
            .collect();
        let mut r = self.splitmix64() % weights.iter().sum::<u64>();
        weights.iter().position(|&w| {
            if r < w {
                return true;
            }
            r -= w;
            false
        })
    }

    fn splitmix64(&mut self) -> u64 {
        self.rng = self.rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.rng;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

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
        let ms = ms.map_or_else(|| "miss".to_owned(), |ms| ms.to_string());
        let line = format!("{}\t{key}\t{ms}\n", stats::date(stats::utc_today()));
        let mut file = (std::fs::OpenOptions::new().create(true).append(true))
            .open(log)
            .with_context(ctx)?;
        file.write_all(line.as_bytes()).with_context(ctx)
    }

    pub fn over(&self) -> bool {
        self.cur.is_none()
    }

    pub fn panel(&self) -> (String, &'static str) {
        let Some(cur) = &self.cur else {
            return (String::new(), "");
        };
        let slot = self.answers.len() + usize::from(!cur.named);
        let task = &POOL[cur.task];
        let text = if cur.named { task.tutor } else { task.drill };
        (format!(" Drill {slot}/{}", self.n), text)
    }

    pub fn summary(&self) -> String {
        let secs = self.started.elapsed().as_secs();
        let n = self.answers.len();
        let mut out = format!(
            "drill: {n} task{} in {}:{:02}\n",
            plural(n),
            secs / 60,
            secs % 60
        );
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
        let w = weak
            .iter()
            .map(|w| POOL[w.2].key.len() + 1)
            .fold(SUMMARY_KEY_COLUMN, usize::max);
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
        _ => Some(u32::midpoint(v[mid - 1], v[mid])),
    }
}

fn drill(app: &mut App) -> Option<&mut Drill> {
    app.tutor.as_mut()?.drill.as_mut()
}

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
    let ms = u32::try_from(since.elapsed().as_millis()).unwrap_or(u32::MAX);
    let Some(d) = drill(app) else {
        return;
    };
    let (said, logged) = match (named, hit) {
        (true, _) => ("\u{2713}".to_owned(), Ok(())),
        (false, true) => (
            format!("\u{2713} {:.1} s", f64::from(ms) / 1000.0),
            d.answer(task, Some(ms)),
        ),
        (false, false) => ("miss".to_owned(), d.answer(task, None)),
    };
    let set = if named || hit {
        next(app)
    } else {
        run(app, task, true)
    };
    app.message = match logged.and(set) {
        Ok(()) => said,
        Err(e) => format!("{said}  {}", crate::app::error_text(&e)),
    };
}

pub fn next(app: &mut App) -> Result<()> {
    let Some(d) = drill(app) else {
        return Ok(());
    };
    if let Some(task) = d.pick() {
        run(app, task, false)
    } else {
        d.cur = None;
        Ok(())
    }
}

fn run(app: &mut App, task: usize, named: bool) -> Result<()> {
    let set = fresh(app).and_then(|()| set_up(app, &POOL[task]));
    if set.is_err()
        && REVIEW_LESSONS.contains(&POOL[task].key)
        && let Some(d) = drill(app)
    {
        d.no_review = true;
        return next(app);
    }
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

    fn task(key: &str) -> usize {
        POOL.iter().position(|t| t.key == key).unwrap()
    }

    fn drill(n: usize) -> Drill {
        let mut d = Drill::new(n, None, None, stats::utc_today()).unwrap();
        d.rng = 7;
        d
    }

    fn session(d: &mut Drill, missed: &[usize]) -> Vec<&'static str> {
        let mut asked = Vec::new();
        while let Some(t) = d.pick() {
            let ms = (!missed.contains(&asked.len())).then_some(1000);
            asked.push(POOL[t].key);
            d.answer(t, ms).unwrap();
        }
        asked
    }

    fn in_keys_order() -> Vec<&'static str> {
        stats::ACTIONS
            .iter()
            .filter_map(|a| POOL.iter().find(|t| t.key == a.name))
            .map(|t| t.key)
            .collect()
    }

    #[test]
    fn a_weight_is_the_work_factor_times_the_drill_factor() {
        let dir = dir("weights");
        drop(std::fs::remove_dir_all(&dir));
        std::fs::create_dir_all(&dir).unwrap();
        let today = stats::utc_today();
        let ago = |days| stats::date(today - days);
        let mut keys = format!(
            "{}\td\t2\t5\n{}\tu\t2\t0\n{}\tu\t50\t0\n",
            ago(1),
            ago(1),
            ago(30)
        );
        for k in ["o", "[", "]", "}", "{"] {
            _ = writeln!(keys, "{}\t{k}\t3\t1", ago(29));
        }
        std::fs::write(dir.join("keys.tsv"), keys).unwrap();
        let mut log = String::new();
        for ms in [5000; 200].into_iter().chain([1000; 100]) {
            _ = writeln!(log, "{}\tn\t{ms}", ago(9));
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
                _ = writeln!(log, "{}\t{key}\t{ms}", ago(2));
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
    fn a_log_line_neither_a_time_nor_a_miss_or_of_a_key_no_task_trains_is_dropped() {
        assert_eq!(
            parse(
                "2026-01-01\td\t900\nnot a date\to\tmiss\n2026-01-01\tgone\t900\n\
                 2026-01-01\td\tfast\n2026-01-01\td\n\n"
            ),
            [("d", Some(900)), ("o", None)]
        );
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
        assert_eq!(
            session(&mut drill(20), &[]),
            in_keys_order()[..20],
            "with nothing attempted, the first session is the start of the tour"
        );
    }

    #[test]
    fn with_no_sample_review_the_drill_asks_no_review_task() {
        let mut d = drill(200);
        d.no_review = true;
        let asked = session(&mut d, &[0, 1, 2, 3]);
        assert_eq!(asked.len(), 200);
        assert!(
            asked.iter().all(|k| !REVIEW_LESSONS.contains(k)),
            "{asked:?}"
        );
    }

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

    #[test]
    fn a_miss_comes_back_three_tasks_later_within_the_session() {
        let asked = session(&mut drill(20), &[0, 3, 17]);
        let k = in_keys_order();
        let mut want = vec![k[0], k[1], k[2], k[0], k[3], k[4], k[0]];
        want.extend(&k[5..18]);
        assert_eq!(asked, want);
    }

    fn on_screen(a: &App) -> (String, &'static str) {
        a.tutor.as_ref().unwrap().drill.as_ref().unwrap().panel()
    }

    fn drill_app(name: &str, n: usize) -> (App, PathBuf) {
        let mut a = app(name, (80, 24));
        let log = dir(name).with_extension("tsv");
        drop(std::fs::remove_file(&log));
        let d = Drill::new(n, None, Some(log.clone()), stats::utc_today()).unwrap();
        a.tutor.as_mut().unwrap().drill = Some(d);
        (a, log)
    }

    fn logged(log: &Path) -> Vec<String> {
        let text = std::fs::read_to_string(log).unwrap_or_default();
        text.lines()
            .map(|l| l.split_once('\t').unwrap().1.to_owned())
            .collect()
    }

    fn clean_up(name: &str, log: &Path) {
        drop(std::fs::remove_dir_all(dir(name)));
        drop(std::fs::remove_file(log));
    }

    #[test]
    fn a_log_that_cannot_be_written_says_why_in_a_few_words() {
        let (mut a, log) = drill_app("drill-io", 2);
        std::fs::create_dir_all(&log).unwrap();
        let t = task("Edit: Alt+Backspace");
        run(&mut a, t, false).unwrap();
        press(&mut a, &"<BS>".repeat(11));
        assert_eq!(
            a.message,
            format!("miss  {}: is a directory", log.display())
        );
        drop(std::fs::remove_dir_all(&log));
        clean_up("drill-io", &log);
    }

    #[test]
    fn a_task_done_another_way_is_a_miss_and_its_redo_is_not_logged() {
        let (mut a, log) = drill_app("drill-miss", 2);
        let t = task("Edit: Alt+Backspace");
        run(&mut a, t, false).unwrap();
        assert_eq!(on_screen(&a), (" Drill 1/2".to_owned(), POOL[t].drill));
        press(&mut a, &"<BS>".repeat(11));
        assert_eq!(a.message, "miss");
        assert_eq!(logged(&log), ["Edit: Alt+Backspace\tmiss"]);
        assert_eq!(on_screen(&a), (" Drill 1/2".to_owned(), POOL[t].tutor));
        assert!(!(POOL[t].done)(&a), "the redo starts from the task's start");
        press(&mut a, POOL[t].answer);
        assert_eq!(a.message, "\u{2713}");
        assert_eq!(logged(&log).len(), 1, "the redo is logged");
        let o = task("o");
        assert_eq!(on_screen(&a), (" Drill 2/2".to_owned(), POOL[o].drill));
        press(&mut a, POOL[o].answer);
        assert!(
            a.message.starts_with("\u{2713} ") && a.message.ends_with(" s"),
            "{}",
            a.message
        );
        assert!(logged(&log)[1].starts_with("o\t"), "{:?}", logged(&log));
        assert!(a.tutor.as_ref().unwrap().drill.as_ref().unwrap().over());
        assert!(
            a.pressed.is_empty() && a.missed.is_empty(),
            "the drill reaches the key stats"
        );
        clean_up("drill-miss", &log);
    }

    #[test]
    fn the_help_opened_is_a_miss_and_an_alias_is_a_hit() {
        let (mut a, log) = drill_app("drill-help", 20);
        run(&mut a, task("d"), false).unwrap();
        press(&mut a, "?<Esc>d");
        assert_eq!(a.message, "miss");
        press(&mut a, "d");
        run(&mut a, task("o"), false).unwrap();
        a.key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL));
        press(&mut a, "store<Enter>");
        assert!(a.message.starts_with("\u{2713} "), "{}", a.message);
        let lines = logged(&log);
        assert_eq!(lines[0], "d\tmiss");
        assert!(
            lines[1]
                .strip_prefix("o\t")
                .is_some_and(|ms| ms.parse::<u32>().is_ok())
        );
        clean_up("drill-help", &log);
    }

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
        d.answer(task("Edit: Alt+Backspace"), None).unwrap();
        assert!(
            d.summary()
                .contains("\n  Edit: Alt+Backspace  1 miss     Delete a word\n"),
            "a key longer than the column still has two spaces after it: {}",
            d.summary()
        );
    }
}
