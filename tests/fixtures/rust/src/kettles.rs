use std::collections::HashMap;

pub struct Kettle;

impl Kettle {
    pub fn boil(&self) {}
}

pub struct Urn;

impl Urn {
    pub fn boil(&self) {}
}

pub fn heat(by_name: HashMap<String, Kettle>, row: Vec<Kettle>, grid: HashMap<String, Vec<Urn>>) {
    by_name.get("a").unwrap().boil();
    //                        ^ d: picker src/kettles.rs:6, src/kettles.rs:12; want src/kettles.rs:6 (#1469)
    row[0].boil();
    //     ^ d: src/kettles.rs:6
    grid.get("a").unwrap()[0].boil();
    //                        ^ d: picker src/kettles.rs:6, src/kettles.rs:12; want src/kettles.rs:12 (#1469)
}
