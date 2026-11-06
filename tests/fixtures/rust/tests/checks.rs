use crate::kit::Gauge;

pub fn checks_read() -> usize {
    let g: Gauge = Gauge;
    //             ^ d: tests/kit.rs:1
    g.read()
    //^ d: tests/kit.rs:4
}
