mod kit;

use crate::kit::Gauge;

fn main() {
    let g: Gauge = Gauge;
    //             ^ d: tests/bench/kit.rs:1
    g.read();
    //^ d: tests/bench/kit.rs:4
}
