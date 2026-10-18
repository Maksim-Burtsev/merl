use super::kettle::{steep, Pour, Spill};
use super::{Kettle, Pot};
use crate::tea::cups::{decant, Strainer};

pub fn shadowed() -> u32 {
    let steep = |m: u32| m * 2;
    steep(3)
//  ^ d: src/tea/brew.rs:6
}

pub fn shadowed_by_value() -> u32 {
    let steep = 4;
    steep + 1
//  ^ d: src/tea/brew.rs:12
}

pub fn parameter(steep: u32) -> u32 {
    steep
//  ^ d: src/tea/brew.rs:17
}

pub fn imported() -> u32 {
    steep(5)
//  ^ d: src/tea/kettle.rs:50
}

pub fn inherent(k: &Kettle) -> u32 {
    k.pour()
//    ^ d: src/tea/kettle.rs:12
}

pub fn by_trait(k: &Kettle) -> u32 {
    Pour::pour(k) + <Kettle as Pour>::pour(k)
//  ^ d: src/tea/kettle.rs:24
//        ^ d: src/tea/kettle.rs:38
//                                    ^ d: src/tea/kettle.rs:38
}

pub fn two_traits(p: &Pot) -> u32 {
    Pour::pour(p) + Spill::pour(p) + <Pot as Spill>::pour(p)
//        ^ d: src/tea/cups.rs:5
//                         ^ d: src/tea/teapot.rs:8
//                                                   ^ d: src/tea/teapot.rs:8
}

pub fn bound<T: Spill>(t: &T) -> u32 {
    t.pour()
//    ^ d: src/tea/kettle.rs:34
}

pub fn twice(k: &Kettle) -> u32 {
    k.twice()
//    ^ d: src/tea/kettle.rs:27
}

pub fn reexports() -> u32 {
    let k = Kettle::new(1);
//          ^ d: src/tea/kettle.rs:1
//                  ^ d: src/tea/kettle.rs:6
    let p = Pot { leaves: 2 };
//          ^ d: src/tea/teapot.rs:3
//                ^ d: src/tea/teapot.rs:4
    let q = crate::tea::Pot { leaves: 3 };
//                      ^ d: src/tea/teapot.rs:3
    k.water + p.leaves + q.leaves + k.refill().water
//    ^ d: src/tea/kettle.rs:2
//              ^ d: src/tea/teapot.rs:4
//                                    ^ d: src/tea/kettle.rs:16
//                                             ^ d: src/tea/kettle.rs:2
}

pub fn generated() -> u32 {
    let c = super::kettle::Caddy;
//                         ^ d: src/tea/kettle.rs:66
    c.rinse()
//    ^ d: src/tea/kettle.rs:59
}

pub fn closure(k: &Kettle) -> Vec<u32> {
    let cups = [1u32, 2];
    cups.iter().map(|water| water + k.water).collect()
//                          ^ d: src/tea/brew.rs:81
//                                    ^ d: src/tea/kettle.rs:2
}

pub fn elsewhere(k: &Kettle) -> bool {
    k.whistle()
//    ^ d: src/tea/cups.rs:11
}

pub fn strings() -> u32 {
    let _s = Strainer;
//           ^ d: src/tea/cups.rs:16
    decant()
//  ^ d: src/tea/cups.rs:18
}

pub struct Celsius(pub u32);
pub struct Fahrenheit(pub u32);

impl From<Fahrenheit> for Celsius {
    fn from(f: Fahrenheit) -> Self {
        Celsius(f.0)
    }
}

impl From<Celsius> for Fahrenheit {
    fn from(c: Celsius) -> Self {
        Fahrenheit(c.0)
    }
}

pub fn convert(c: Celsius) -> Fahrenheit {
    From::from(c)
//        ^ d: picker src/tea/brew.rs:102, src/tea/brew.rs:108
}
