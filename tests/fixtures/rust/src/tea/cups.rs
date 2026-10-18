use super::kettle::{Kettle, Pour};
use super::teapot::Teapot;

impl Pour for Teapot {
    fn pour(&self) -> u32 {
        self.leaves * 2
    }
}

impl Kettle {
    pub fn whistle(&self) -> bool {
        self.water > 90
    }
}

pub struct Strainer;

pub fn decant() -> u32 {
    3
}
