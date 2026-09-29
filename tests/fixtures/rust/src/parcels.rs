//! #363: `Type::new` of the type this file declares, beside a namesake in another crate.

pub struct Parcel;

impl Parcel {
    pub fn new() -> Parcel {
        Parcel
    }
}

pub fn wrap() -> Parcel {
    Parcel::new()
    //      ^ d: src/parcels.rs:6
    //      status: new → Parcel::new (via Parcel)
}
