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
    //   ^ d: src/carts.rs:7
}

impl Wagon {
    pub fn hitched(&self) -> Sled {
        Sled
    }
}

impl Sled {
    pub fn hitched(&self) -> Wagon {
        Wagon
    }
}

pub fn closure_called() -> u32 {
    let pull = |_n: u32| {
        wagon()
    };
    pull(1).roll()
    //      ^ d: src/carts.rs:7
}

pub fn closure_written() -> u32 {
    let pull = move |_n: u32| -> Sled { Sled };
    pull(1).roll()
    //      ^ d: src/carts.rs:19
}

pub fn closure_chained(shed: Wagon) -> u32 {
    let pull = |_n: u32| shed.hitched();
    pull(1).roll()
    //      ^ d: src/carts.rs:19
}

pub fn closure_param_shadows(shed: Wagon) -> u32 {
    let pull = |shed: Sled| shed.hitched();
    pull(Sled).roll()
    //         ^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4
}

pub fn closure_shadowed(shed: Wagon) -> u32 {
    let pull = |_n: u32| shed.hitched();
    let pull = wagon;
    pull().roll()
    //     ^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4
}

pub fn closure_with_statements(shed: Wagon) -> u32 {
    let pull = |_n: u32| {
        let s = shed.hitched();
        s
    };
    pull(1).roll()
    //      ^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4
}
