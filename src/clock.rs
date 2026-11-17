use jiff::{Timestamp, Zoned, tz::TimeZone};
use serde::{Deserialize, Deserializer, de::Error};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clock(Option<String>);

const DEFAULT: &str = "%H:%M";
const KNOWN: &str = "HIklMSpRTraAbBdemyY%";

impl Default for Clock {
    fn default() -> Self {
        Self(Some(DEFAULT.to_owned()))
    }
}

impl<'de> Deserialize<'de> for Clock {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            On(bool),
            Format(String),
        }
        match Raw::deserialize(d)
            .map_err(|_| D::Error::custom("clock is false, true or a format such as \"%H:%M\""))?
        {
            Raw::On(false) => Ok(Self(None)),
            Raw::On(true) => Ok(Self::default()),
            Raw::Format(f) => check(&f).map(|()| Self(Some(f))).map_err(D::Error::custom),
        }
    }
}

fn check(format: &str) -> Result<(), String> {
    if format.chars().any(char::is_control) {
        return Err("clock: one line, no tabs or newlines".into());
    }
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            continue;
        }
        match chars.next() {
            Some(k) if KNOWN.contains(k) => {}
            Some(k) => {
                return Err(format!(
                    "clock: %{k} is not a time field; use %H %M %S, %I %p for 12 hours, %a %d %b for the date"
                ));
            }
            None => return Err("clock: a lone % at the end; write %% for a percent sign".into()),
        }
    }
    Ok(())
}

impl Clock {
    pub fn now(&self) -> Option<String> {
        let format = self.0.as_deref()?;
        show(format, &now())
    }
}

fn show(format: &str, at: &Zoned) -> Option<String> {
    let mut chrono_r = String::with_capacity(format.len());
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            chrono_r.push(c);
            continue;
        }
        match chars.next() {
            Some('r') => chrono_r.push_str("%I:%M:%S %p"),
            k => chrono_r.extend(std::iter::once('%').chain(k)),
        }
    }
    jiff::fmt::strtime::format(chrono_r, at).ok()
}

#[cfg(not(target_family = "wasm"))]
fn now() -> Zoned {
    use std::sync::OnceLock;
    static FIXED: OnceLock<Option<Timestamp>> = OnceLock::new();
    FIXED
        .get_or_init(|| {
            std::env::var("MERL_CLOCK_AT")
                .ok()
                .and_then(|s| s.parse().ok())
                .and_then(|secs| Timestamp::from_second(secs).ok())
        })
        .map_or_else(Zoned::now, |at| at.to_zoned(TimeZone::system()))
}

#[cfg(target_family = "wasm")]
fn now() -> Zoned {
    let zone = std::env::var("MERL_UTC_OFFSET")
        .ok()
        .and_then(|s| s.parse().ok())
        .and_then(|secs| jiff::tz::Offset::from_seconds(secs).ok())
        .map_or(TimeZone::UTC, TimeZone::fixed);
    Timestamp::now().to_zoned(zone)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    struct C {
        #[serde(default)]
        clock: Clock,
    }

    fn parse(toml: &str) -> Result<Clock, String> {
        toml::from_str::<C>(toml)
            .map(|c| c.clock)
            .map_err(|e| e.to_string())
    }

    #[test]
    fn the_setting_reads_off_on_and_a_format() {
        assert_eq!(parse("").unwrap(), Clock::default());
        assert_eq!(parse("clock = false").unwrap(), Clock(None));
        assert_eq!(parse("clock = true").unwrap(), Clock::default());
        assert_eq!(
            parse("clock = \"%I:%M %p\"").unwrap(),
            Clock(Some("%I:%M %p".into()))
        );
        assert!(
            parse("clock = \"%H:%Q\"")
                .unwrap_err()
                .contains("%Q is not a time field")
        );
        assert!(parse("clock = \"%H:%\"").unwrap_err().contains("lone %"));
        assert!(
            parse("clock = \"%H\\n%M\"")
                .unwrap_err()
                .contains("one line")
        );
        assert!(parse("clock = 5").unwrap_err().contains("false, true"));
    }

    #[test]
    fn every_field_prints_as_chrono_printed_it() {
        let all: String = KNOWN.chars().flat_map(|k| ['[', '%', k, ']']).collect();
        let all = format!("{all}a%r%%r%%%r x");
        for (secs, chrono) in [
            (
                0,
                "[00][12][ 0][12][00][00][AM][00:00][00:00:00][12:00:00 AM][Thu][Thursday][Jan][January][01][ 1][01][70][1970][%]a12:00:00 AM%r%12:00:00 AM x",
            ),
            (
                52_325,
                "[14][02][14][ 2][32][05][PM][14:32][14:32:05][02:32:05 PM][Thu][Thursday][Jan][January][01][ 1][01][70][1970][%]a02:32:05 PM%r%02:32:05 PM x",
            ),
            (
                18_429,
                "[05][05][ 5][ 5][07][09][AM][05:07][05:07:09][05:07:09 AM][Thu][Thursday][Jan][January][01][ 1][01][70][1970][%]a05:07:09 AM%r%05:07:09 AM x",
            ),
            (
                1_700_000_000,
                "[22][10][22][10][13][20][PM][22:13][22:13:20][10:13:20 PM][Tue][Tuesday][Nov][November][14][14][11][23][2023][%]a10:13:20 PM%r%10:13:20 PM x",
            ),
        ] {
            let at = Timestamp::from_second(secs)
                .unwrap()
                .to_zoned(TimeZone::UTC);
            assert_eq!(show(&all, &at).unwrap(), chrono, "{secs}");
        }
    }

    #[test]
    fn every_known_field_formats_as_a_time() {
        let at = Timestamp::from_second(14 * 3600 + 32 * 60 + 5)
            .unwrap()
            .to_zoned(TimeZone::UTC);
        let show = |f: &str| show(f, &at).unwrap();
        assert_eq!(show("%H:%M"), "14:32");
        assert_eq!(show("%I:%M %p"), "02:32 PM");
        assert_eq!(show("%H:%M:%S"), "14:32:05");
        for k in KNOWN.chars() {
            let f = format!("%{k}");
            assert!(check(&f).is_ok());
            assert!(!show(&f).contains('%') || k == '%', "{f} -> {}", show(&f));
        }
    }

    #[test]
    fn a_new_time_asks_for_a_redraw_and_the_same_time_does_not() {
        let mut app = crate::app::App::new(
            std::path::PathBuf::from("/work"),
            crate::tree::Tree::default(),
            Vec::new(),
            crate::buffer::Buffer::from_bytes(std::path::PathBuf::from("/work/a.py"), b""),
            None,
        );
        app.clock_format = Clock(Some("%Y".into()));
        assert!(app.tick_clock());
        assert!(!app.tick_clock());
        app.clock = Some("1999".into());
        assert!(app.tick_clock());
        app.clock_format = Clock(None);
        assert!(app.tick_clock() && app.clock.is_none());
    }

    #[test]
    fn no_clock_draws_nothing() {
        assert_eq!(Clock(None).now(), None);
    }
}
