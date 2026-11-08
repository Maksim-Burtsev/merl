pub struct Goblet;
impl Goblet {
    pub fn quaff(&self) {}
}
pub struct Stand<T>(pub T);
pub struct Rack<Goblet> {
    held: Goblet,
    //    ^ d: src/goblets.rs:6
}
pub fn fill<Goblet>(x: Goblet) -> Goblet {
//                     ^ d: src/goblets.rs:10
    x
}
impl<Goblet> Stand<Goblet> {
    pub fn get(&self, x: Goblet) {}
    //                   ^ d: src/goblets.rs:14
}
pub fn wrapped<Goblet>(
    x: Goblet,
    // ^ d: src/goblets.rs:18
) {
}
pub fn plain(x: Goblet) {}
//              ^ d: src/goblets.rs:1
pub fn valued<Goblet>(x: Goblet) -> crate::goblets::Goblet {
    Goblet
    //^ d: src/goblets.rs:1
}
pub fn outer<Goblet>(x: Goblet) {
    fn inner() -> Goblet {
    //            ^ d: src/goblets.rs:1
        crate::goblets::Goblet
    }
    let s = "fn h<Goblet>(y: Goblet)";
    // fn h<Goblet>(y: Goblet)
    //                 ^ d: src/goblets.rs:1
}
pub trait Toaster {
    fn clink(&self);
}
impl Toaster for Goblet {
    fn clink(&self) {}
}
pub fn probe_fill<Goblet>(x: Goblet) {
    x.quaff();
    //^ d: none
}
pub fn probe_unbound<T>(x: T) {
    x.quaff();
    //^ d: none
}
pub fn probe_bounded<T: Toaster>(x: T) {
    x.clink();
    //^ d: src/goblets.rs:39
}
pub struct Shelf<T> {
    held: T,
    cup: Goblet,
}
impl<T> Shelf<T> {
    pub fn probe_field(&self) {
        self.held.quaff();
        //        ^ d: src/goblets.rs:3; want none (#1438)
        self.cup.quaff();
        //       ^ d: src/goblets.rs:3
    }
}
