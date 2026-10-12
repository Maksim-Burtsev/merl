use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Deserializer, de::Error};

#[derive(Debug, Clone, PartialEq)]
pub struct Clock(Option<String>);

pub const DEFAULT: &str = "%H:%M";
const KNOWN: &str = "HIklMSpRTraAbBdemyYZ%";

impl Default for Clock {
    fn default() -> Self {
        Self(Some(DEFAULT.into()))
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
        match Raw::deserialize(d).map_err(|_| {
            D::Error::custom("clock is false, true or a format such as \"%H:%M\"")
        })? {
            Raw::On(false) => Ok(Self(None)),
            Raw::On(true) => Ok(Self::default()),
            Raw::Format(f) => check(&f).map(|()| Self(Some(f))).map_err(D::Error::custom),
        }
    }
}

fn check(format: &str) -> Result<(), String> {
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
        Some(strftime(format, now_secs()))
    }
}

fn now_secs() -> i64 {
    static START: OnceLock<(Instant, Option<i64>)> = OnceLock::new();
    let (start, fake) = START.get_or_init(|| {
        let fake = std::env::var("MERL_PROTO_EPOCH")
            .ok()
            .and_then(|s| s.parse().ok());
        (Instant::now(), fake)
    });
    match fake {
        Some(epoch) => epoch + start.elapsed().as_secs() as i64,
        None => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64),
    }
}

fn strftime(format: &str, secs: i64) -> String {
    let Ok(format) = std::ffi::CString::new(format) else {
        return String::new();
    };
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let t: libc::time_t = secs as libc::time_t;
    let mut buf = [0u8; 128];
    let n = unsafe {
        if libc::localtime_r(&t, &mut tm).is_null() {
            return String::new();
        }
        libc::strftime(
            buf.as_mut_ptr().cast(),
            buf.len(),
            format.as_ptr(),
            &tm,
        )
    };
    String::from_utf8_lossy(&buf[..n]).into_owned()
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
        assert!(parse("clock = \"%H:%Q\"").unwrap_err().contains("%Q"));
        assert!(parse("clock = 5").unwrap_err().contains("false, true"));
    }

    #[test]
    fn strftime_formats_local_time() {
        unsafe { std::env::set_var("TZ", "UTC") };
        assert_eq!(strftime("%H:%M", 14 * 3600 + 32 * 60 + 5), "14:32");
        assert_eq!(strftime("%I:%M %p", 14 * 3600 + 32 * 60), "02:32 PM");
        assert_eq!(strftime("%T", 14 * 3600 + 32 * 60 + 5), "14:32:05");
    }
}
