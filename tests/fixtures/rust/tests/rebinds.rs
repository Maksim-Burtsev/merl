pub struct Caps;
pub struct Grain;

impl Caps {
    pub fn get(&self) -> usize {
        0
    }
    pub fn into_grain(&self) -> Grain {
        Grain
    }
}

impl Grain {
    pub fn get(&self) -> usize {
        1
    }
}

fn grain() -> Option<Grain> {
    Some(Grain)
}

pub fn unit() {
    let held = Caps;
    held.get();
    //   ^ d: !jump; want tests/rebinds.rs:5 (#1302)
}

pub fn unit_mut() {
    let mut held = Grain;
    held.get();
    //   ^ d: !jump; want tests/rebinds.rs:14 (#1302)
}

pub fn rebind(held: Caps) {
    let held = held.into_grain();
    held.get();
    //   ^ d: !jump; want tests/rebinds.rs:14 (#1302)
}

pub fn let_else(held: Caps) {
    let Some(held) = grain() else { return };
    held.get();
    //   ^ d: !jump; want tests/rebinds.rs:14 (#1302)
}

pub fn let_else_wrapped(held: Caps) {
    let Some(held) = grain() else {
        return;
    };
    held.get();
    //   ^ d: !jump; want tests/rebinds.rs:14 (#1302)
}

pub fn for_ref(held: Caps, v: Vec<Grain>) {
    for held in &v {
        held.get();
        //   ^ d: tests/rebinds.rs:14
    }
}

pub fn closure(held: Caps) {
    let f = |held: Grain| {
        held.get()
        //   ^ d: tests/rebinds.rs:14
    };
}
