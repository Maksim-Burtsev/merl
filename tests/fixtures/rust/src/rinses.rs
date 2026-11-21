use crate::servings::Crock;

pub fn dip_each(f: impl Fn(&Crock)) {
    f(&Crock);
}

// A closure passed to a function that declares its input (`impl Fn(&Crock)`), and `max_by_key`
// over a `Vec<Crock>`, land on `Crock`'s method, not `Wok`'s namesake (#1488).
pub fn rinse() {
    dip_each(|x| { x.stew(); });
    //               ^ d: src/servings.rs:5
    let crocks: Vec<Crock> = vec![Crock];
    let _ = crocks.iter().max_by_key(|x| x.stew());
    //                                     ^ d: src/servings.rs:5
}
