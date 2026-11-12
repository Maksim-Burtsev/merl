// A case two enums of the project share, written without its enum (#1448).
use crate::corvid::Corvid;
use crate::corvid::Corvid::*;

pub fn perch(bird: Corvid) -> u8 {
    match bird {
        Rook => 1,
     // ^ d: src/corvid.rs:3
        Raven => 0,
    }
}

pub fn roost() -> Corvid {
    let _qualified = Corvid::Rook;
                          // ^ d: src/corvid.rs:3
    Rook
 // ^ d: src/corvid.rs:3
}

pub fn local(bird: crate::chessman::Chessman) -> u8 {
    use crate::chessman::Chessman::*;
    match bird {
        Rook => 1,
     // ^ d: src/chessman.rs:2
        Pawn => 0,
    }
}
