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
