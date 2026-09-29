//! #370: a field read without a call, the words of an attribute, an enum variant.

pub struct Stats {
    searches: u64,
}

impl Stats {
    pub fn searches(&self) -> u64 {
        self.searches
        //   ^ d: src/stock.rs:4
    }
}

pub struct Config {
    capacity: usize,
}

pub struct Builder {
    config: Config,
}

impl Builder {
    pub fn capacity(&mut self, capacity: usize) -> &mut Builder {
        self.config.capacity = capacity;
        //   ^ d: src/stock.rs:19
        //          ^ d: src/stock.rs:15
        self
    }

    pub fn fresh(
        size: usize,
    ) -> Builder {
        let config = Config {
            capacity: size,
        };
        Builder { config }
    }
}

pub struct Tester;

impl Tester {
    pub fn test(&self) {}
}

#[cfg(test)]
//    ^ d: none
mod tests {
    #[test]
    //^ d: none
    fn works() {}
}

pub enum Mode {
    Auto,
    //^ d: src/stock.rs:55
    Never,
}

pub fn auto(m: Mode) -> bool {
    matches!(m, Mode::Auto)
    //                ^ d: src/stock.rs:55
}

pub struct Regex;

pub enum Strategy {
    Literal(String),
    Regex(u32),
}

impl Strategy {
    pub fn is_regex(&self) -> bool {
        use self::Strategy::*;
        match *self {
            Regex(_) => true,
            //^ d: src/stock.rs:69
            Literal(_) => false,
        }
    }
}

pub fn plain() -> Regex {
    Regex
    //^ d: src/stock.rs:65
}

pub mod probe {
    use super::Strategy::*;

    pub fn literal(
        s: &super::Strategy,
    ) -> bool {
        matches!(s, Literal(_))
        //          ^ d: src/stock.rs:68
    }
}

pub fn sample() -> &'static str {
    "\
line (one
#[derive(
"
}

#[test]
//^ d: none
fn after_a_string() {}

#[cfg_attr(
    test,
    //^ d: none
    derive(Debug)
    //     ^ d: none
)]
pub struct Probe;

// #346, #370: an attribute macro may name project code, in a value string or a nested list.
pub fn default_port() -> u16 {
    80
}

#[serde(default = "default_port")]
//                 ^ d: src/stock.rs:119
#[pool(capacity = 3)]
//     ^ d: none
#[error("Tester")]
//       ^ d: none
#[doc = "Tester"]
//       ^ d: none
#[diesel(belongs_to(Tester))]
//                  ^ d: src/stock.rs:40
#[enum_dispatch(Tester)]
//              ^ d: src/stock.rs:40
pub struct Row;
