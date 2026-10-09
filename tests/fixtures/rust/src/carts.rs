//! #611: the type of a local, read only when every binding in scope says the same.

#[derive(Default)]
pub struct Wagon;

impl Wagon {
    pub fn roll(&self) -> u32 {
        1
    }

    pub fn make() -> Sled {
        Sled
    }
}

pub struct Sled;

impl Sled {
    pub fn roll(&self) -> u32 {
        2
    }
}

pub fn wagon() -> Wagon {
    Wagon
}

#[allow(non_upper_case_globals)]
pub static roller: fn() -> Wagon = wagon;

pub fn shadowed() -> u32 {
    let cart: Wagon = Wagon;
    [Sled].iter().map(|cart: &Sled| cart.roll()).sum::<u32>() + cart.roll()
    //                                   ^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4
}

pub fn defaulted() -> u32 {
    let cart = Wagon::default();
    cart.roll()
    //   ^ d: src/carts.rs:7
}

pub fn made() -> u32 {
    let cart = Wagon::make();
    cart.roll()
    //   ^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4
}

pub fn closure() -> u32 {
    let wagon = || Sled;
    let cart = wagon();
    cart.roll()
    //   ^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4
}

pub fn rolled() -> u32 {
    let cart = roller();
    cart.roll()
    //   ^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4
}

impl Sled {
    pub fn twin(&self) -> u32 {
        let other: Self = Sled;
        other.roll()
        //    ^ d: src/carts.rs:19
    }
}

pub fn make<F>(g: F) -> Wagon
where
    F: Fn() -> Sled,
{
    Wagon
}

pub fn bounded() -> u32 {
    let cart = make(|| Sled);
    cart.roll()
    //   ^ d: src/carts.rs:19; want src/carts.rs:7 (#785)
}
