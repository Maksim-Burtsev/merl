use crate::goblets::Shelf;

impl<T> Shelf<T> {
    pub fn probe_far(&self) {
        self.held.quaff();
        //        ^ d: src/goblets.rs:3; want none (#1438)
    }
}
