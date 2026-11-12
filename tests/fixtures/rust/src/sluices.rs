pub struct Tumbler;
pub struct Beaker;

pub trait Lather {
    fn foam(&self) {}
}

impl Lather for Tumbler {}

pub trait Sluice {
    fn sluice(&self);
}

impl<T: Lather> Sluice for T {
    fn sluice(&self) {}
}

pub trait Douse {
    fn douse(&self);
}

impl<T: Lather + ?Sized> Douse for T {
    fn douse(&self) {}
}

pub trait Soak {
    fn soak(&self);
}

impl<T: Douse> Soak for T {
    fn soak(&self) {}
}

pub fn rinse(t: Tumbler, b: Beaker) {
    t.sluice();
    //^ d: src/sluices.rs:15
    t.douse();
    //^ d: src/sluices.rs:23
    t.foam();
    //^ d: src/sluices.rs:5
    b.soak();
    //^ d: src/sluices.rs:27
}
