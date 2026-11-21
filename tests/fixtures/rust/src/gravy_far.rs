use crate::gravy::{make_marker, Boat, Gravy, Marker};

// `Gravy` is a struct with named fields declared in another file, so a `let` or a parameter named
// like it still binds a variable (#1515).
#[allow(non_snake_case)]
pub fn local() -> i32 {
    let Gravy = Boat::make();
    Gravy.drizzle()
  //^ d: src/gravy_far.rs:7
    //    ^ d: src/gravy.rs:20
}

#[allow(non_snake_case)]
pub fn param(Gravy: &Boat) -> i32 {
    Gravy.drizzle()
    //    ^ d: src/gravy.rs:20
}

// `Marker` is a unit struct from another file: `let Marker = …` binds nothing.
#[allow(non_snake_case)]
pub fn unit() -> i32 {
    let Marker = make_marker();
    let _g: Option<Gravy> = None;
    Marker.ping()
  //^ d: src/gravy.rs:56
}
