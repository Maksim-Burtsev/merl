use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use serde::Deserialize;

use crate::TokenBucket;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Limits {
    pub rate_per_minute: u32,
    pub burst: u32,
}

impl Limits {
    pub fn new_bucket(&self, now: f64) -> TokenBucket {
        TokenBucket {
            capacity: f64::from(self.burst),
            refill_per_second: f64::from(self.rate_per_minute) / 60.0,
            tokens: f64::from(self.burst),
            updated_at: now,
        }
    }

    /// How long an untouched bucket takes to fill up again; after that the store may drop it.
    pub fn idle_ttl(&self) -> Duration {
        let seconds = (f64::from(self.burst) * 60.0 / f64::from(self.rate_per_minute)).ceil();
        Duration::from_secs(seconds as u64 + 1)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LimitsConfig {
    pub default: Limits,
    #[serde(default)]
    pub overrides: HashMap<String, Limits>,
}

impl LimitsConfig {
    pub fn for_key(&self, api_key: &str) -> Limits {
        self.overrides.get(api_key).copied().unwrap_or(self.default)
    }
}

pub fn load_config(path: &Path) -> anyhow::Result<LimitsConfig> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}
