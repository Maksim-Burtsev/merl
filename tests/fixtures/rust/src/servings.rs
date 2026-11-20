pub struct Crock;
pub struct Wok;

impl Crock {
    pub fn stew(&self) {}
}

impl Wok {
    pub fn stew(&self) {}
}

pub struct Kit {
    pub crock: Crock,
    pub wok: Wok,
}

pub fn serve_pair() -> (Crock, Wok) {
    (Crock, Wok)
}

pub fn serve_kit() -> Kit {
    Kit { crock: Crock, wok: Wok }
}

pub fn unpack() {
    let (c, w) = serve_pair();
    w.stew();
//    ^ d: src/servings.rs:9
    c.stew();
//    ^ d: src/servings.rs:5
    let Kit { crock, wok } = serve_kit();
    wok.stew();
//      ^ d: src/servings.rs:9
    crock.stew();
//        ^ d: src/servings.rs:5
}

pub struct Rack(pub Crock, pub Wok);

pub struct Shelf<T> {
    pub held: T,
}

pub enum Pot {
    Lidded { wok: Crock },
}

pub fn unpack_more(rack: Rack, shelf: Shelf<Wok>, kit: Kit) {
    let Rack(front, back) = rack;
    back.stew();
//       ^ d: src/servings.rs:9
    front.stew();
//        ^ d: src/servings.rs:5
    let Kit { wok: w2, .. } = kit;
    w2.stew();
//     ^ d: src/servings.rs:9
    let Shelf { held } = shelf;
    held.stew();
//       ^ d: !jump
}

pub fn unpack_variant(pot: Pot) {
    let Pot::Lidded { wok } = pot;
    wok.stew();
//      ^ d: src/servings.rs:5
}

pub fn serve_three() -> (Wok, Wok, Crock) {
    (Wok, Wok, Crock)
}

// A tuple pattern typed by its annotation, bound to a tuple of unit structs, or past a `..` (#1484).
pub fn unpack_written() {
    let (c, w): (Crock, Wok) = serve_pair();
    w.stew();
//    ^ d: src/servings.rs:9
    c.stew();
//    ^ d: src/servings.rs:5
    let (p, q) = (Crock, Wok);
    q.stew();
//    ^ d: src/servings.rs:9
    p.stew();
//    ^ d: src/servings.rs:5
    let (.., z) = serve_three();
    z.stew();
//    ^ d: src/servings.rs:5
    let (y, ..) = serve_three();
    y.stew();
//    ^ d: src/servings.rs:9
}

// A struct pattern in a parameter types each name by its field (#1484).
pub fn unpack_param(Kit { crock, wok }: Kit) {
    wok.stew();
//      ^ d: src/servings.rs:9
    crock.stew();
//        ^ d: src/servings.rs:5
}

pub fn unpack_rack(Rack(front, back): Rack) {
    back.stew();
//       ^ d: src/servings.rs:9
}

pub fn unpack_inline(Kit { crock, wok }: Kit) { wok.stew() }
//                                                  ^ d: src/servings.rs:9
