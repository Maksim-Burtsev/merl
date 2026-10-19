//! #1045: a type imported through a crate of the workspace that hands another crate on (`pub extern crate other as wound;`).

use facade::wound::Bale;

pub fn baled() -> u32 {
    Bale::new().wind()
    //          ^ d: other/src/lib.rs:48
}
