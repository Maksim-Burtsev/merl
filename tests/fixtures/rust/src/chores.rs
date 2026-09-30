//! #363: the namesakes of `errands`, out of its sight without a path.

fn helper() -> u32 {
    2
}

pub fn mkdirp() -> u32 {
    helper()
}

// #363, after the review: the namesakes of what `errands` declares below.
pub struct Tally;

impl crate::errands::Tally {
    pub fn stamp() -> u32 {
        1
    }
}

pub fn tally() -> u32 {
    1
}

macro_rules! tally {
    () => {
        1
    };
}

macro_rules! late {
    () => {
        1
    };
}

fn regex() -> u32 {
    1
}
