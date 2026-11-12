use super::Cauldron;
use super::gleam::Gleam;

impl super::gleam::Glow for Cauldron {
    fn gleam(&self) {}
}

pub fn glint(pot: Cauldron) {
    pot.gleam();
    //  ^ d: src/cauldrons/sheen.rs:4
}
