use crate::chessman::Chessman::{self, *};

pub fn advance(piece: Chessman) -> u8 {
    match piece {
        Rook => 1,
     // ^ d: none; want src/chessman.rs:2 (#1448)
        Chessman::Pawn => 0,
    }
}
