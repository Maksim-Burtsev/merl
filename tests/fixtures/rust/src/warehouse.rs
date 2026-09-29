pub const BANNER: &str = r#"
pub fn weigh(grams: u32) -> u32 {
pub struct Basket {
"#;

pub const NOTE: &str = "
pub fn settle(total: u32) -> u32 {
";

pub struct Courier {
    pub name: String,
}

impl Courier {
    pub fn new(name: &str) -> Courier {
        Courier { name: name.into() }
    }
    pub fn unwrap(self) -> Courier {
        self
    }
}

pub fn weigh(grams: u32) -> u32 {
    grams / 1000
}

pub mod depot {
    pub fn open() -> bool {
        true
    }
}
