pub struct Kettle {
    pub water: u32,
}

impl Kettle {
    pub fn new(water: u32) -> Self {
        Self { water }
//      ^ d: none; want src/tea/kettle.rs:5 (#987)
//             ^ d: src/tea/kettle.rs:6
    }

    pub fn pour(&self) -> u32 {
        self.water
    }

    pub fn refill(&self) -> Self {
//                          ^ d: none; want src/tea/kettle.rs:5 (#987)
        Self::new(self.water + 1)
//      ^ d: none; want src/tea/kettle.rs:5 (#987)
//            ^ d: picker src/tea/kettle.rs:6, …; want src/tea/kettle.rs:6 (#987)
    }
}

pub trait Pour {
    fn pour(&self) -> u32;

    fn twice(&self) -> u32 {
        self.pour() * 2
//           ^ d: picker src/tea/kettle.rs:12, src/tea/kettle.rs:25, src/tea/kettle.rs:34, src/tea/kettle.rs:38, src/tea/cups.rs:5, src/tea/teapot.rs:8; want src/tea/kettle.rs:25 (#989)
    }
}

pub trait Spill {
    fn pour(&self) -> u32;
}

impl Pour for Kettle {
    fn pour(&self) -> u32 {
        self.water + 1
//           ^ d: src/tea/kettle.rs:2
    }
}

/// Steeps the leaves. A recipe, not code:
///
/// ```ignore
/// pub fn decant() -> u32 { 0 }
/// impl Pour for Caddy {}
/// ```
pub fn steep(minutes: u32) -> u32 {
    minutes
}

macro_rules! vessel {
    ($name:ident) => {
        pub struct $name;

        impl $name {
            pub fn rinse(&self) -> u32 {
                0
            }
        }
    };
}

vessel!(Caddy);
//^ d: src/tea/kettle.rs:54
