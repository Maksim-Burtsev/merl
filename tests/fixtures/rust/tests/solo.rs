mod probe {
    pub struct Gauge;
}

use crate::probe::Gauge;

pub fn solo() {
    let _g = Gauge;
    //       ^ d: tests/solo.rs:2
}

mod common;
