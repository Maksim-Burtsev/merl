pub struct Brazier {
    pub heat: u32,
}

impl Brazier {
    pub const MAX: u32 = 9;
    pub fn new() -> Self {
        Brazier { heat: 0 }
    }
    pub fn light(&self) {}
    pub fn hotter(&self) -> u32 {
        Self::MAX
        //    ^ d: picker src/braziers.rs:6, src/braziers.rs:20, src/infusion/leaves.rs:7; want src/braziers.rs:6 (#1489)
    }
}

pub struct Grate;

impl Grate {
    pub const MAX: u32 = 3;
    pub fn new() -> Self {
        Grate
    }
    pub fn light(&self) {}
}

pub fn stoke() -> u32 {
    Brazier::new().light();
    //             ^ d: src/braziers.rs:10
    let b = Brazier::new();
    //               ^ d: src/braziers.rs:7
    b.light();
    Brazier::MAX
    //       ^ d: src/braziers.rs:6
}
