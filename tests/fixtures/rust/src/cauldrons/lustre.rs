use super::Cauldron;

pub trait Burnish {
    fn burnish(&self);
}

impl Burnish for Cauldron {
    fn burnish(&self) {}
}
