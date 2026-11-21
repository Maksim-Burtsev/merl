pub mod own;

pub struct Mug;
pub struct Pan;

impl Mug {
    pub fn get(&self) -> u8 {
        1
    }
}

impl Pan {
    pub fn get(&self) -> u8 {
        2
    }
}
