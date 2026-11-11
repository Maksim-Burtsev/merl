use crate::goblets::Shelf;

impl<T> Shelf<T> {
    pub fn probe_far(&self) {
        self.held.quaff();
        //        ^ d: none
    }
}
impl Shelf<crate::goblets::Goblet> {
    pub fn probe_far_concrete(&self) {
        self.held.quaff();
        //        ^ d: src/goblets.rs:3
    }
}
impl<T: crate::goblets::Toaster> Shelf<T> {
    pub fn probe_far_bounded(&self) {
        self.held.clink();
        //        ^ d: src/goblets.rs:39
        self.cup.quaff();
        //       ^ d: src/goblets.rs:3
    }
}
