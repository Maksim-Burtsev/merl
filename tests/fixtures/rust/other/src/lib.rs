//! A second crate the shop does not depend on (#363, #358).

pub struct Parcel;

impl Parcel {
    pub fn new() -> Parcel {
        Parcel
    }
}
