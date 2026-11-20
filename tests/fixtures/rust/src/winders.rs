pub struct Winder;

impl Winder {
    pub fn reel(&self) -> Result<other::Reel, String> {
        Ok(other::Reel)
    }
}

// A chain through a return type written with another crate of the workspace
// (`Result<other::Reel, _>`), wrapped over lines: `wind` lands on `Reel`'s, not on `Spool`'s or
// `Bale`'s namesake (#1469).
pub fn wind_up(w: &Winder) -> Result<u32, String> {
    let n = w
        .reel()?
        .wind();
    //   ^ d: other/src/lib.rs:24
    Ok(n)
}

// `coil` is `pub(crate)` in `other`: the typed chain refuses it as the by-name lookup does (#1469).
pub fn coil_up(w: &Winder) -> Result<u32, String> {
    let n = w.reel()?.coil();
    //                ^ d: none
    Ok(n)
}
