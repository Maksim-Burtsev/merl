/* A block comment that reads like code declares nothing:
pub struct Tariff {
*/

pub const RATE_CAP: u32 = 100;
pub static CURRENCY: &str = "EUR";

pub struct Tariff {
    pub base: u32,
}

impl Tariff {
    pub fn rate(&self) -> u32 {
        1
    }
    pub fn describe(&self) -> String {
        "tariff".into()
    }
}

pub struct Coupon;

impl Coupon {
    pub fn rate(&self) -> u32 {
        2
    }
    pub fn describe(&self) -> String {
        "coupon".into()
    }
}

pub trait Priced {
    fn price(&self) -> u32;
}

pub enum Offer {
    Plain,
    Cut(u32),
}

pub type Money = u32;

pub union Bits {
    pub whole: u32,
    pub half: u16,
}

macro_rules! cents {
    ($x:expr) => {
        $x * 100
    };
}
pub(crate) use cents;

pub fn discount(total: u32) -> u32 {
    total.min(RATE_CAP) - 1
}

pub async fn settle(total: u32) -> u32 {
    total
}

pub struct Debug;
