// A unit struct named by its path types the `let` it is bound to (#1487); `Laver` is the namesake.
pub struct Ewer;
impl Ewer {
    pub fn swill(&self) {}
}
pub struct Laver;
impl Laver {
    pub fn swill(&self) {}
}

pub fn by_crate_path() {
    let e = crate::ewers::Ewer;
    e.swill();
    //^ d: src/ewers.rs:4
}

pub fn by_self_path() {
    let e = self::Laver;
    e.swill();
    //^ d: src/ewers.rs:8
}

pub fn by_lowercase_path() {
    let e = crate::ewers::swill;
    e.swill();
    //^ d: picker src/ewers.rs:4, src/ewers.rs:8
}
