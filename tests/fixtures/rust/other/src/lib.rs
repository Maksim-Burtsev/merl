//! A second crate the shop does not depend on (#363, #358).

pub struct Parcel;

impl Parcel {
    pub fn new() -> Parcel {
        Parcel
    }
}

pub trait Shine {
    fn polish(&self) -> u32;
}

impl Shine for shop::rules::Lamp {
    fn polish(&self) -> u32 {
        1
    }
}
