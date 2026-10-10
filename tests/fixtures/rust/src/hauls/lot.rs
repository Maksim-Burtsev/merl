use crate::carts::Wagon;

pub fn towed(hauler: Wagon) -> u32 {
    hauler.roll()
    //     ^ d: src/carts.rs:7
}

pub fn hued() -> u32 {
    use crate::tints::Tint::*;
    let _ = Coral;
    //      ^ d: picker src/rules.rs:55, src/tints.rs:9
    0
}
