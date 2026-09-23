//! Key stats (#207): how often each action of [`KEYS`] is pressed in real work, a line per UTC
//! day and action in `~/.local/state/merl/keys.tsv`, which `merl --keys` prints.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{KEYS, plural};

/// The second side of these rows is VS Code's key for the first: a press counts toward the
/// first, and they are no action of their own. A VS Code habit is no blind spot, the action gets
/// done.
const ALIASES: &[&str] = &["Ctrl+E", "Ctrl+F", "F12", "Shift+F12", "Ctrl+G"];

/// A side of a `KEYS` row: what the stats count.
pub struct Action {
    pub name: String,
    /// The row's description.
    pub what: &'static str,
    /// The row's other side, when it is one of [`ALIASES`].
    alias: Option<String>,
}

/// Every action, in `KEYS` order.
pub static ACTIONS: LazyLock<Vec<Action>> = LazyLock::new(|| {
    let mut actions = Vec::new();
    for (keys, what) in KEYS {
        let mut sides = sides(keys);
        let alias = sides.pop_if(|s| ALIASES.contains(&s.as_str()));
        actions.extend(sides.into_iter().map(|name| Action {
            name,
            what,
            alias: alias.clone(),
        }));
    }
    actions
});

/// The sides of a `KEYS` key column. The second takes what the first has in front of its key,
/// the prefix and the modifiers, unless it spells its own: `Tree: Up / Down` is `Tree: Down`,
/// `Alt+Shift+Left / Right` is `Alt+Shift+Right`, `Edit: Alt+Backspace / Alt+Delete` is
/// `Edit: Alt+Delete`.
fn sides(keys: &str) -> Vec<String> {
    let Some((first, second)) = keys.split_once(" / ") else {
        return vec![keys.to_string()];
    };
    let prefix = first.find(": ").map_or("", |i| &first[..i + 2]);
    let head = first.rfind('+').map_or(prefix, |i| &first[..=i]);
    let second = match second.contains('+') {
        true => format!("{prefix}{second}"),
        false => format!("{head}{second}"),
    };
    vec![first.to_string(), second]
}

/// A key as `KEYS` spells it: `Ctrl+Shift+Left`, `PgUp`, `Shift+F12`, `N`.
pub fn name(key: KeyEvent) -> String {
    let mut s = String::new();
    for (m, word) in [
        (KeyModifiers::CONTROL, "Ctrl+"),
        (KeyModifiers::ALT, "Alt+"),
        (KeyModifiers::SHIFT, "Shift+"),
    ] {
        if key.modifiers.contains(m) {
            s += word;
        }
    }
    match key.code {
        KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            s.push(c.to_ascii_uppercase());
        }
        KeyCode::Char(c) => s.push(c),
        KeyCode::F(n) => _ = write!(s, "F{n}"),
        KeyCode::PageUp => s += "PgUp",
        KeyCode::PageDown => s += "PgDn",
        // `Up`, `Enter`, `Esc`, `Home`: the variant's own name.
        code => _ = write!(s, "{code:?}"),
    }
    s
}

/// The action the key `key`, spelled as [`name`] does and prefixed with where it was pressed,
/// counts toward: its own, its primary's for an alias, `Arrows` for a bare arrow.
pub fn action(key: &str) -> Option<&'static str> {
    let key = match key {
        "Up" | "Down" | "Left" | "Right" => "Arrows",
        _ => key,
    };
    ACTIONS
        .iter()
        .find(|a| a.name == key || a.alias.as_deref() == Some(key))
        .map(|a| a.name.as_str())
}

/// Presses by UTC day and action.
type Rows = BTreeMap<(i64, &'static str), u64>;

/// Built from the home directory the way `config.toml` is, no `XDG_*`.
pub fn path() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".local/state/merl/keys.tsv"))
}

/// Days since 1970-01-01, UTC: the local day would need a time zone database.
pub fn today() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    (secs / 86_400) as i64
}

/// `YYYY-MM-DD` of a day since 1970-01-01: Howard Hinnant's `civil_from_days`.
fn date(day: i64) -> String {
    let z = day + 719_468;
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The day of a `YYYY-MM-DD`, `days_from_civil`; `None` for anything [`date`] would not write.
fn day(s: &str) -> Option<i64> {
    let mut parts = s.splitn(3, '-').map(|p| p.parse::<i64>().ok());
    let (Some(Some(y)), Some(Some(m)), Some(Some(d))) = (parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let y = y - i64::from(m <= 2);
    let (era, yoe) = (y.div_euclid(400), y.rem_euclid(400));
    let doy = (153 * if m > 2 { m - 3 } else { m + 9 } + 2) / 5 + d - 1;
    let n = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    (date(n) == s).then_some(n)
}

/// `YYYY-MM-DD<TAB>action<TAB>count` lines. One that is not, or names an action no longer in
/// `KEYS`, is dropped. Columns after the count are for later: missed keys will add one.
fn parse(text: &str) -> Rows {
    let mut rows = Rows::new();
    for line in text.lines() {
        let mut cols = line.split('\t');
        let (Some(day), Some(action), Some(n)) = (
            cols.next().and_then(day),
            cols.next()
                .and_then(|name| ACTIONS.iter().find(|a| a.name == name)),
            cols.next().and_then(|n| n.parse::<u64>().ok()),
        ) else {
            continue;
        };
        *rows.entry((day, action.name.as_str())).or_default() += n;
    }
    rows
}

fn load(path: &Path) -> Result<Rows> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(parse(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Rows::new()),
        Err(e) => Err(e).with_context(|| format!("{}", path.display())),
    }
}

/// Adds a session's presses to the file under `today`. The file is read again right before and
/// replaced whole through a rename, so a merl quitting after this one keeps both numbers and a
/// reader never sees half a file; only two quitting in the same instant can lose one's. A file
/// that cannot be read is left as it is.
pub fn add(path: &Path, today: i64, pressed: &HashMap<&'static str, u64>) -> Result<()> {
    if pressed.is_empty() {
        return Ok(());
    }
    let ctx = || format!("{}", path.display());
    let mut rows = load(path)?;
    for (action, n) in pressed {
        *rows.entry((today, action)).or_default() += n;
    }
    let mut text = String::new();
    for ((day, action), n) in &rows {
        _ = writeln!(text, "{}\t{action}\t{n}", date(*day));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(ctx)?;
    }
    let tmp = path.with_extension(format!("tsv.{}", std::process::id()));
    std::fs::write(&tmp, text).with_context(ctx)?;
    std::fs::rename(&tmp, path).with_context(ctx)
}

/// `merl --keys`.
pub fn report(path: &Path, today: i64) -> Result<String> {
    Ok(table(&load(path)?, today))
}

/// Every action with its presses in the last 30 days, in all, and the day of the last one,
/// strongest first: the never pressed come last, right above the prompt.
fn table(rows: &Rows, today: i64) -> String {
    let mut keys: Vec<(&str, u64, u64, Option<i64>, &str)> = ACTIONS
        .iter()
        .map(|a| (a.name.as_str(), 0, 0, None, a.what))
        .collect();
    for (&(day, action), n) in rows {
        if let Some(k) = keys.iter_mut().find(|k| k.0 == action) {
            if today - day < 30 {
                k.1 += n;
            }
            k.2 += n;
            k.3 = k.3.max(Some(day));
        }
    }
    keys.sort_by_key(|k| std::cmp::Reverse((k.1, k.2)));
    let last = |day: Option<i64>| match day.map(|d| today - d) {
        None => "never".to_string(),
        Some(..=0) => "today".to_string(),
        Some(n) => format!("{n} day{} ago", plural(n as usize)),
    };
    let head = ["key", "30d", "all", "last", "what"].map(String::from);
    let cells: Vec<[String; 5]> = std::iter::once(head)
        .chain(keys.iter().map(|&(name, month, all, day, what)| {
            [
                name.into(),
                month.to_string(),
                all.to_string(),
                last(day),
                what.into(),
            ]
        }))
        .collect();
    let w = |i: usize| cells.iter().map(|c| c[i].len()).max().unwrap_or(0);
    let (w0, w1, w2, w3) = (w(0), w(1), w(2), w(3));
    cells
        .iter()
        .map(|[key, month, all, last, what]| {
            format!("{key:<w0$}  {month:>w1$}  {all:>w2$}  {last:<w3$}  {what}\n")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    /// Each side of a `KEYS` row is an action; the second side takes the first side's prefix
    /// and modifiers, and the five aliases fold into their primaries.
    #[test]
    fn keys_split_into_66_actions() {
        let names: Vec<&str> = ACTIONS.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names.len(), 66, "{names:?}");
        assert_eq!(names.iter().collect::<HashSet<_>>().len(), names.len());
        for name in [
            "[",
            "]",
            "n",
            "N",
            "Ctrl+Z",
            "Ctrl+Y",
            "Arrows",
            "Tree: Up",
            "Tree: Down",
            "Alt+Shift+Left",
            "Alt+Shift+Right",
            "Edit: Alt+Backspace",
            "Edit: Alt+Delete",
            "Picker: Enter",
        ] {
            assert!(names.contains(&name), "{name}");
        }
        for (alias, primary) in [
            ("Ctrl+E", "o"),
            ("Ctrl+F", "/"),
            ("F12", "d"),
            ("Shift+F12", "u"),
            ("Ctrl+G", ":"),
        ] {
            assert!(!names.contains(&alias), "{alias}");
            assert_eq!(action(alias), Some(primary), "{alias}");
        }
        assert_eq!(action("Up"), Some("Arrows"));
        assert_eq!(action("Tree: Up"), Some("Tree: Up"));
        assert_eq!(action("Enter"), Some("Enter"));
        assert_eq!(action("x"), None);
    }

    #[test]
    fn days_are_utc_dates() {
        assert_eq!(day("1970-01-01"), Some(0));
        assert_eq!(day("2026-09-23"), Some(20_719));
        assert_eq!(day("2024-02-29"), Some(19_782));
        for bad in ["2026-02-29", "2026-13-01", "2026-9-23", "20260923", "today"] {
            assert_eq!(day(bad), None, "{bad}");
        }
    }

    /// Two merl instances quitting one after the other both keep their numbers; a line of an
    /// action no longer in `KEYS`, or no line at all, is dropped.
    #[test]
    fn two_writers_add_up_and_unknown_actions_are_dropped() {
        let dir = std::env::temp_dir().join(format!("merl-keys-merge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("state/merl/keys.tsv");
        let today = day("2026-09-23").unwrap();
        add(&file, today, &HashMap::from([("d", 2), ("]", 1)])).unwrap();
        let old = std::fs::read_to_string(&file).unwrap();
        std::fs::write(
            &file,
            format!("2026-09-20\td\t5\n2026-09-20\tgone\t9\n2026-09-20\tUp\t2\nnot a line\n{old}"),
        )
        .unwrap();
        add(&file, today, &HashMap::from([("d", 3)])).unwrap();
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "2026-09-20\td\t5\n2026-09-23\t]\t1\n2026-09-23\td\t5\n"
        );
        // A file that cannot be read is not written over.
        std::fs::write(&file, b"\xff\n").unwrap();
        assert!(add(&file, today, &HashMap::from([("d", 1)])).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"\xff\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `merl --keys`: strongest first by the last 30 days, the never pressed last, every action
    /// listed.
    #[test]
    fn the_weakest_keys_are_printed_last() {
        let rows = parse(
            "2026-01-01\td\t1592\n2026-09-23\td\t212\n2026-08-01\t}\t15\n2026-09-20\t}\t2\n\
             2026-08-13\tv\t3\n",
        );
        let out = table(&rows, day("2026-09-23").unwrap());
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(
            lines[..4],
            [
                "key                  30d   all  last         what",
                "d                    212  1804  today        Go to definition of the word under the \
                 cursor, or its implementations",
                "}                      2    17  3 days ago   Previous / next paragraph (blank line)",
                "v                      0     3  41 days ago  Select the word, then the line, then \
                 the paragraph",
            ]
        );
        assert_eq!(lines.len(), 1 + 66);
        assert!(lines[4..].iter().all(|l| l.contains("  never  ")), "{out}");
        assert_eq!(
            lines[4],
            "o                      0     0  never        Open a file (fuzzy)"
        );
    }
}
