use super::kettle::Spill;

pub struct Teapot {
    pub leaves: u32,
}

impl Spill for Teapot {
    fn pour(&self) -> u32 {
        self.leaves
    }
}

pub const LABEL: &str = r#"
pub struct Strainer;
impl Spill for Kettle {}
"#;
