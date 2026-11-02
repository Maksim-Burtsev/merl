pub struct Mesh {
    pub start: usize,
    pub end: usize,
}

pub struct Grain {
    pub end: usize,
}

pub trait Holes {
    fn get(&self, i: usize) -> Option<Mesh>;
}

pub trait Grains {
    fn get(&self, i: usize) -> Option<Grain>;
}

pub trait Sieve {
    type Holes: Holes;
    type Grains: Grains + Clone;
    type Loose;

    fn widest(&self, holes: &mut Self::Holes) -> usize {
        let m = holes.get(0).unwrap();
        m.end
        //^ d: src/sieves.rs:3
    }

    fn first(&self, holes: &Self::Holes) -> Option<Mesh> {
        holes.get(0)
        //    ^ d: src/sieves.rs:11
    }

    fn finest(&self, grains: &Self::Grains) -> usize {
        let g = grains.get(0).unwrap();
        g.end
        //^ d: src/sieves.rs:7
    }

    fn loose(&self, loose: &Self::Loose) -> usize {
        let l = loose.get(0).unwrap();
        l.end
        //^ d: picker src/sieves.rs:3, src/sieves.rs:7
    }
}

pub fn generic<H: Holes>(holes: &H) -> usize {
    let m = holes.get(0).unwrap();
    m.end
    //^ d: src/sieves.rs:3
}

pub struct Riddle;

pub trait Shaker {
    type Holes;

    fn shake(&self, holes: &Self::Holes) -> usize;
}

impl Shaker for Riddle {
    type Holes = Vec<Grain>;

    fn shake(&self, holes: &Self::Holes) -> usize {
        let m = holes.get(0).unwrap();
        m.end
        //^ d: !jump
    }
}
