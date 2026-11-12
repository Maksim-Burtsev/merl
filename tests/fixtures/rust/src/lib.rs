//! The shop of #307: every `d` case carries its answer in a comment under it.

pub mod basket;
pub mod pricing;
pub mod stock;
pub mod warehouse;
pub mod texts;
pub mod chores;
pub mod errands;
pub mod parcels;
pub mod locals;
pub mod receivers;
pub mod rules;
pub mod tints;
pub mod carts;
pub mod hauls;
pub mod tea;
pub mod looms;
pub mod heddles;
pub mod bales;
pub mod infusion;
pub mod sieves;
pub mod husks;
pub mod corvid;
pub mod chessman;
pub mod rookery;
pub mod board;
pub mod flagons;
pub mod kylix;
pub mod vessels;
pub mod feast;
pub mod ewers;
pub mod braziers;

use ewers::{Ewer, Laver};

pub fn rinse_all() {
    let e = Ewer;
    e.swill();
    //^ d: src/ewers.rs:4
    let l = ewers::Laver;
    l.swill();
    //^ d: src/ewers.rs:8
    let _ = Laver;
}
