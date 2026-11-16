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
        //        ^ d: none
        self.cup.quaff();
        //       ^ d: src/goblets.rs:3
    }
}
impl<U> Shelf<U> {
    pub fn probe_renamed(&self) {
        self.held.quaff();
        //        ^ d: none
    }
}
impl<T> Toaster for Shelf<T> {
    fn clink(&self) {
        self.held.quaff();
        //        ^ d: none
    }
}
impl Shelf<Goblet> {
    pub fn probe_concrete(&self) {
        self.held.quaff();
        //        ^ d: src/goblets.rs:3
    }
}
pub struct Cellar<T: Toaster> {
    held: T,
}
impl<T: Toaster> Cellar<T> {
    pub fn probe_bounded(&self) {
        self.held.clink();
        //        ^ d: src/goblets.rs:39
    }
}
pub struct Vault<T> {
    held: T,
}
impl<T> Vault<T>
where
    T: Toaster,
{
    pub fn probe_where(&self) {
        self.held.clink();
        //        ^ d: src/goblets.rs:39
    }
}
pub fn probe_below<T>(x: T) -> u8
where
    T: Toaster,
{
    x.clink();
    //^ d: src/goblets.rs:39
    0
}
pub fn probe_wrapped<P, T>(
    &mut self,
    x: T,
    y: P,
) -> u8
where
    P: Toaster,
    T: Toaster,
{
    x.clink();
    //^ d: src/goblets.rs:39
    0
}
pub trait Sip {
    fn sip_it(&self) {}
}
impl<T> Sip for T {}
pub fn probe_blanket<T>(x: T) {
    x.sip_it();
    //^ d: src/goblets.rs:129
}
pub struct Tankard<'a, T> {
    held: &'a T,
    boxed: Box<T>,
}
impl<'a, T> Tankard<'a, T> {
    pub fn probe_ref(&self) {
        self.held.quaff();
        //        ^ d: none
        self.boxed.quaff();
        //         ^ d: none
    }
}
pub struct Decanter<T> {
    held: T,
}
impl<T>
    Decanter<T>
{
    pub fn probe_wrapped_impl(&self) {
        self.held.quaff();
        //        ^ d: none
    }
}
pub struct Carafe<T> {
    held: T,
}
pub unsafe trait Decant {
    fn decant(&self);
}
unsafe impl<T> Decant for Carafe<T> {
    fn decant(&self) {
        self.held.quaff();
        //        ^ d: none
    }
}
pub trait Siphon {
    fn siphon(&self);
}
impl<T> Siphon
    for Decanter<T>
{
    fn siphon(&self) {
        self.held.quaff();
        //        ^ d: none
    }
}
impl<
    T: Toaster,
> Carafe<T> {
    pub fn probe_wrapped_params(&self) {
        self.held.clink();
        //        ^ d: src/goblets.rs:39
    }
}
pub fn probe_cloned<Goblet: Clone>(x: Goblet) {
    x.quaff();
    //^ d: none
}
pub fn probe_cloned_t<T: Clone>(x: T) {
    x.quaff();
    //^ d: src/goblets.rs:3
}
pub fn probe_cloned_where<Goblet>(x: Goblet)
where
    Goblet: Clone,
{
    x.quaff();
    //^ d: none
}
pub fn probe_cloned_toaster<Goblet: Clone + Toaster>(x: Goblet) {
    x.clink();
    //^ d: src/goblets.rs:39
}
pub fn probe_cloned_std<Goblet: Clone>(x: Goblet) -> Goblet {
    x.clone()
    //^ d: none
}
pub fn probe_cloned_line<Goblet: Clone>(x: Goblet) { x.quaff(); }
//                                                     ^ d: none
pub fn probe_shine<M: other::Shine>(x: M) -> u32 {
    x.polish()
    //^ d: picker other/src/lib.rs:12, other/src/lib.rs:16, src/rules.rs:63
}
pub fn probe_as_ref<P: AsRef<str>>(p: P) -> usize {
    p.as_ref().len()
    //^ d: none
}
pub trait Glaze: Toaster {}
pub fn probe_glaze<S: Glaze>(x: S) {
    x.clink();
    //^ d: src/goblets.rs:39
}
