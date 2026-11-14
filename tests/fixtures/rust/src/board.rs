use crate::chessman::Chessman::{self, *};

pub fn advance(piece: Chessman) -> u8 {
    match piece {
        Rook => 1,
     // ^ d: src/chessman.rs:2
        Chessman::Pawn => 0,
    }
}
