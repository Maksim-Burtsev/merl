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
