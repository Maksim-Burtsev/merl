//! #611: a bare name behind a glob `use` of an enum, beside the names that hide its variants.

use self::Tint::*;
use lamps::Amber;

pub enum Tint {
    Teal,
    Amber,
    Coral,
}

pub struct Teal;

pub fn teal() -> Teal {
    Teal
    // ^ d: src/tints.rs:12
}

pub fn amber() -> Amber {
    Amber
    // ^ d: src/rules.rs:52
}

pub fn coral() -> Tint {
    Coral
    // ^ d: src/tints.rs:9
}

pub fn hauled<Wagon: Default>(c: crate::carts::Wagon, _w: Wagon) -> u32 {
    c.roll()
    //^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4
}
