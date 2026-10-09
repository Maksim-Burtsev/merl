use crate::carts::Wagon;

pub fn towed(hauler: Wagon) -> u32 {
    hauler.roll()
    //     ^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4; want src/carts.rs:7 (#783)
}

pub fn hued() -> u32 {
    use crate::tints::Tint::*;
    let _ = Coral;
    //      ^ d: picker src/rules.rs:55, src/tints.rs:9
    0
}
