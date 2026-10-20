use chrono::{DateTime, Utc};
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
        Some(now().format(format).to_string())
    }
}

#[cfg(not(target_family = "wasm"))]
fn now() -> DateTime<chrono::Local> {
    use chrono::Local;
    use std::sync::OnceLock;
    static FIXED: OnceLock<Option<DateTime<Utc>>> = OnceLock::new();
    FIXED
        .get_or_init(|| {
            std::env::var("MERL_CLOCK_AT")
                .ok()
                .and_then(|s| s.parse().ok())
                .and_then(|secs| DateTime::from_timestamp(secs, 0))
        })
        .map_or_else(Local::now, |at| at.with_timezone(&Local))
}

#[cfg(target_family = "wasm")]
fn now() -> DateTime<chrono::FixedOffset> {
    use chrono::Offset;
    let zone = std::env::var("MERL_UTC_OFFSET")
        .ok()
        .and_then(|s| s.parse().ok())
        .and_then(chrono::FixedOffset::east_opt)
        .unwrap_or_else(|| Utc.fix());
    Utc::now().with_timezone(&zone)
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
    fn every_known_field_formats_as_a_time() {
        let at = DateTime::from_timestamp(14 * 3600 + 32 * 60 + 5, 0).unwrap();
        let show = |f: &str| at.format(f).to_string();
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
