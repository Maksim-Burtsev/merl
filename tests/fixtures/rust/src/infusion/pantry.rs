pub use super::leaves::Leaf as Sheet;
pub use super::leaves::{sip, Infuser};

pub mod shelf {
    pub use super::super::leaves::*;
}

pub mod north {
    pub fn kind() -> &'static str {
        "north"
    }
}

pub mod south {
    pub fn kind() -> &'static str {
        "south"
    }
}
