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
    //   ^ d: tests/rebinds.rs:5
}

pub fn unit_mut() {
    let mut held = Grain;
    held.get();
    //   ^ d: tests/rebinds.rs:14
}

pub fn rebind(held: Caps) {
    let held = held.into_grain();
    held.get();
    //   ^ d: tests/rebinds.rs:14
}

pub fn let_else(held: Caps) {
    let Some(held) = grain() else { return };
    held.get();
    //   ^ d: tests/rebinds.rs:14
}

pub fn let_else_wrapped(held: Caps) {
    let Some(held) = grain() else {
        return;
    };
    held.get();
    //   ^ d: tests/rebinds.rs:14
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

pub struct Pair(pub usize);

impl Pair {
    pub fn get(&self) -> usize {
        self.0
    }
}

fn caps() -> Result<Grain, ()> {
    Ok(Grain)
}

pub fn tuple_struct_fn(held: Caps) {
    let held = Pair;
    held.get();
    //   ^ d: !jump
}

pub fn rebind_undeclared(held: Caps) {
    let held = held.missing();
    held.get();
    //   ^ d: !jump
}

pub fn let_else_wrong_wrapper(held: Caps) {
    let Some(held) = caps() else { return };
    held.get();
    //   ^ d: !jump
}

pub fn let_else_own_line(held: Caps) {
    let Some(held) = grain()
    else {
        return;
    };
    held.get();
    //   ^ d: !jump
}

pub fn rebind_twice(held: Caps) {
    let held = held.into_grain();
    let held = Caps;
    held.get();
    //   ^ d: tests/rebinds.rs:5
}

pub fn two_lets_on_a_line(held: Caps) {
    let held = Caps; let held = held.into_grain();
    held.get();
    //   ^ d: !jump
}

pub fn two_rebinds_on_a_line(held: Caps) {
    let held = held.into_grain(); let held = Caps;
    held.get();
    //   ^ d: !jump
}
