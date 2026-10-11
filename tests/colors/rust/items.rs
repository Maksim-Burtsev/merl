//! Items a Rust file is made of.
#![allow(dead_code)]

extern crate alloc;

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display};
use std::io::Result as IoResult;
use crate::inner::*;

pub mod inner {
    pub(crate) const LIMIT: usize = 1 << 10;
    pub static mut COUNTER: u64 = 0;
}

/// A point on a grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point<T = i32> {
    pub x: T,
    y: T,
}

struct Unit;
struct Wrapper(Vec<u8>, &'static str);

#[repr(u8)]
enum Shape {
    Circle { r: f64 },
    Square(f64),
    Empty = 0,
}

pub trait Area: Display + Send {
    type Output;
    const SIDES: u32;
    fn area(&self) -> Self::Output;
    fn name(&self) -> String {
        String::from("shape")
    }
}

impl<T: Copy + Into<f64>> Point<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub async fn dist<'a>(&'a self, other: &'a Point<T>) -> f64
    where
        T: std::ops::Sub<Output = T>,
    {
        let dx: f64 = (self.x - other.x).into();
        dx.abs()
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unit")
    }
}

type Table = HashMap<String, Box<dyn Fn(i32) -> i32>>;

unsafe fn raw(p: *const u8, q: *mut u8) -> Option<impl Iterator<Item = u8>> {
    union Bits { i: u32, f: f32 }
    None
}
