use crate::kylix::Kylix;
use crate::vessels::{Kantharos, Rhyton, Skyphos};

pub fn feast() {
    let c = Kylix;
    c.libate();
    //^ d: src/kylix.rs:2
    let s = Skyphos;
    s.libate();
    //^ d: src/vessels.rs:4
    let r = Rhyton;
    r.libate();
    //^ d: src/vessels.rs:5
    let k = Kantharos;
    k.libate();
    //^ d: src/vessels.rs:7
}
