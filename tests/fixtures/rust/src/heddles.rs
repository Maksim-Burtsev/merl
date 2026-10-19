//! #1070: `d` on a field where its struct declares it never jumps to a namesake: a module the file imports, the struct's own getter, another type's setter.

use crate::looms::{self, Loom};

pub struct Reed {
    pub heddle: u32,
    //  ^ d: src/heddles.rs:6
    looms: Option<Loom>,
    //^ d: picker src/heddles.rs:32
    shed: u8,
    //^ d: picker src/heddles.rs:17
    pick: u8,
    //^ d: picker src/heddles.rs:27, src/heddles.rs:41
}

impl Reed {
    pub fn shed(&self) -> u8 {
        self.shed
    }
}

pub struct ReedBuilder {
    count: u8,
}

impl ReedBuilder {
    pub fn pick(&mut self, n: u8) -> &mut Self {
        self.count = n;
        self
    }

    pub fn looms(&self) -> Option<Loom> {
        let _ = looms::Bobbin;
        None
    }
}

pub enum Warp {
    Taut { shed: u8 },
    Slack {
        pick: u8,
        //^ d: picker src/heddles.rs:12, src/heddles.rs:27
    },
}

pub fn threaded() -> Reed {
    Reed {
        heddle: 1,
        //^ d: src/heddles.rs:6
        looms: None,
        shed: 2,
        pick: 3,
    }
}
