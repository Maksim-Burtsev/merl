pub struct Leaf {
    pub grade: u32,
    pub name: String,
}

impl Leaf {
    pub const MAX: u32 = 9;

    pub fn new(grade: u32) -> Self {
        Leaf { grade: grade.min(Self::MAX), name: String::new() }
    }

    pub fn len(&self) -> usize {
        self.name.len()
    }

    pub fn trim(&self) -> &str {
        self.name.trim()
    }

    pub fn push(&mut self, c: char) {
        self.name.push(c);
    }
}

pub trait Steeper {
    fn minutes(&self) -> u32 {
        3
    }

    fn label(&self) -> String {
        String::from("steeper")
    }
}

pub trait Timed: Steeper {
    fn alarm(&self) -> u32 {
        self.minutes() * 60
    }
}

impl Steeper for Leaf {
    fn label(&self) -> String {
        self.name.clone()
    }
}

impl Timed for Leaf {}

pub struct Infuser {
    leaves: u32,
    water: u32,
}

impl Infuser {
    pub fn start() -> Self {
        Self { leaves: 0, water: 0 }
    }

    pub fn leaves(mut self, n: u32) -> Self {
        self.leaves = n;
        self
    }

    pub fn water(mut self, n: u32) -> Self {
        self.water = n;
        self
    }

    pub fn finish(self) -> Liquor {
        Liquor { strength: self.leaves * self.water }
    }
}

pub struct Liquor {
    pub strength: u32,
}

impl Liquor {
    pub fn taste(&self) -> u32 {
        self.strength
    }
}

pub fn sip() -> u32 {
    1
}

pub fn gulp() -> u32 {
    2
}

pub type Tin = Leaf;
pub type Kettle = Vec<u32>;
