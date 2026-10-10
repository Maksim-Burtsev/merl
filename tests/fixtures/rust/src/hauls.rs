//! #611: which of the project's types a written type names.

use self::north::Runner;
use self::yard::Wagon;

mod lot;
mod yard;

mod north {
    pub struct Runner;

    impl Runner {
        pub fn glide(&self) -> u32 {
            1
        }
    }
}

mod south {
    pub struct Runner;

    impl Runner {
        pub fn glide(&self) -> u32 {
            2
        }
    }

    pub fn sped(r: Runner) -> u32 {
        r.glide()
        //^ d: picker src/hauls.rs:13, src/hauls.rs:23
    }
}

pub fn slid(s: crate::carts::Sled) -> u32 {
    s.roll()
    //^ d: picker src/carts.rs:7, src/carts.rs:19, src/hauls/yard.rs:4
}

pub fn ran(r: Runner) -> u32 {
    r.glide()
    //^ d: src/hauls.rs:13
}

pub fn yarded(y: Wagon) -> u32 {
    y.roll()
    //^ d: src/hauls/yard.rs:4
}
