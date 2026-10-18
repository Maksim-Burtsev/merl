use std::collections::HashMap;

use anyhow::Context;

/// Token bucket for one API key. Times are Unix seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenBucket {
    pub capacity: f64,
    pub refill_per_second: f64,
    pub tokens: f64,
    pub updated_at: f64,
}

impl TokenBucket {
    pub fn take(&mut self, now: f64) -> bool {
        self.refill(now);
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }

    pub fn retry_after(&mut self, now: f64) -> f64 {
        self.refill(now);
        let missing = 1.0 - self.tokens;
        if missing <= 0.0 {
            return 0.0;
        }
        missing / self.refill_per_second
    }

    fn refill(&mut self, now: f64) {
        let elapsed = (now - self.updated_at).max(0.0);
        self.tokens = (self.tokens + elapsed * self.refill_per_second).min(self.capacity);
        self.updated_at = now;
    }

    pub(crate) fn to_fields(&self) -> [(&'static str, f64); 4] {
        [
            ("capacity", self.capacity),
            ("refill_per_second", self.refill_per_second),
            ("tokens", self.tokens),
            ("updated_at", self.updated_at),
        ]
    }

    pub(crate) fn from_fields(fields: &HashMap<String, f64>) -> anyhow::Result<Self> {
        let field = |name: &str| fields.get(name).copied().with_context(|| format!("missing field {name}"));
        Ok(Self {
            capacity: field("capacity")?,
            refill_per_second: field("refill_per_second")?,
            tokens: field("tokens")?,
            updated_at: field("updated_at")?,
        })
    }
}
