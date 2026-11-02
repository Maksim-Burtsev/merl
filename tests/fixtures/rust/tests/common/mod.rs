use crate::kit::Gauge;

pub fn common() {
    let _g: Gauge = Gauge;
    //              ^ d: picker tests/bench/kit.rs:1, tests/kit.rs:1, tests/solo.rs:2
}
