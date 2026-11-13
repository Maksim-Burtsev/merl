pub struct Gravy {
    pub n: i32,
}

impl Gravy {
    pub fn drizzle(&self) -> i32 {
        self.n
    }
}

pub struct Boat {
    pub n: i32,
}

impl Boat {
    pub fn make() -> Boat {
        Boat { n: 1 }
    }

    pub fn drizzle(&self) -> i32 {
        self.n
    }
}

// A struct with named fields is no value, so a `let` or a parameter named like it binds a
// variable (#1515).
#[allow(non_snake_case)]
pub fn local() -> i32 {
    let Gravy = Boat::make();
    Gravy.drizzle()
  //^ d: src/gravy.rs:29
}

#[allow(non_snake_case)]
pub fn method() -> i32 {
    let Gravy = Boat::make();
    Gravy.drizzle()
    //    ^ d: src/gravy.rs:20
}

#[allow(non_snake_case)]
pub fn param(Gravy: &Boat) -> i32 {
    Gravy.drizzle()
    //    ^ d: src/gravy.rs:20
}
