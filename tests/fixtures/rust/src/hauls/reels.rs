//! A `use` inside a function of an inline `mod` types the name in that function only (the Punchcard review of #1130, finding 1).

pub struct Hoist;

impl Hoist {
    pub fn new() -> Hoist {
        Hoist
    }

    pub fn lift(&self) -> u32 {
        2
    }
}

pub struct Bale;

impl Bale {
    pub fn new() -> Bale {
        Bale
    }

    pub fn wind(&self) -> u32 {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn foreign() -> u32 {
        use other::Bale;
        Bale::new().wind()
        //          ^ d: other/src/lib.rs:48
    }

    fn local() -> u32 {
        Bale::new().wind()
        //          ^ d: src/hauls/reels.rs:22
    }

    fn typed() -> u32 {
        let b: Bale = Bale::new();
        b.wind()
        //^ d: src/hauls/reels.rs:22
    }
}
