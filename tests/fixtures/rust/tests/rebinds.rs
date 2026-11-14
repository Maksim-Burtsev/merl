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
    //   ^ d: tests/rebinds.rs:14
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

pub fn closure_untyped(held: Caps, v: Vec<Grain>) {
    v.iter().for_each(|held| {
        held.get();
        //   ^ d: tests/rebinds.rs:14
    });
}

pub fn closure_bare(held: Caps) {
    let f = |held| held.get();
    //                  ^ d: !jump
}

pub fn closure_wrapped(held: Caps, v: Vec<Grain>) {
    v.iter().for_each(
        |held| {
            held.get();
            //   ^ d: tests/rebinds.rs:14
        },
    );
}

pub fn for_chain(held: Caps, v: Vec<Grain>) {
    for held in v.iter().filter(|g| g.get() > 0) {
        held.get();
        //   ^ d: tests/rebinds.rs:14
    }
}

pub fn if_let_else(held: Caps) {
    let n = if let Some(held) = grain() { 0 } else { held.get() };
    //                                                    ^ d: tests/rebinds.rs:5
}

pub fn if_let_then(held: Caps) {
    let n = if let Some(held) = grain() { held.get() } else { 0 };
    //                                         ^ d: tests/rebinds.rs:14
}

pub fn if_let_wrong_wrapper(held: Caps) {
    let n = if let Some(held) = caps() { held.get() } else { 0 };
    //                                        ^ d: !jump
}

pub fn if_let_after_else(held: Caps) {
    let n = if let Some(held) = grain() { 0 } else { 1 } + held.get();
    //                                                          ^ d: tests/rebinds.rs:5
}

pub fn if_let_else_if(held: Caps) {
    let n = if let Some(held) = grain() { 0 } else if true { held.get() } else { 1 };
    //                                                            ^ d: tests/rebinds.rs:5
}

pub fn if_let_else_rebinds(held: Caps) {
    let n = if let Some(held) = grain() { 0 } else { let held = Grain; held.get() };
    //                                                                      ^ d: !jump; want tests/rebinds.rs:14 (#1302)
}

pub fn if_let_else_for(held: Caps, v: Vec<Grain>) {
    let n = if let Some(held) = grain() { 0 } else { for held in v.iter() { held.get(); } 0 };
    //                                                                           ^ d: !jump; want tests/rebinds.rs:14 (#1302)
}

pub fn if_let_else_while_let(held: Caps) {
    let n = if let Some(held) = grain() { 0 } else { while let Some(held) = grain() { held.get(); } 0 };
    //                                                                                      ^ d: !jump; want tests/rebinds.rs:14 (#1302)
}

pub fn if_let_after_for(held: Caps, v: Vec<Grain>) {
    let n = if let Some(held) = grain() { 0 } else { 1 } + { for held in v.iter() { held.get(); } 0 };
    //                                                                                    ^ d: !jump; want tests/rebinds.rs:14 (#1302)
}

pub fn if_let_then_for(held: Caps, w: Vec<Caps>) {
    if let Some(held) = grain() { for held in w.iter() { held.get(); } }
    //                                                        ^ d: !jump; want tests/rebinds.rs:5 (#1302)
}

pub fn if_let_then_twice(held: Caps) {
    if let Some(held) = grain() { held.get(); held.get(); }
    //                                             ^ d: tests/rebinds.rs:14
}

pub fn if_let_after_used(held: Caps) {
    if let Some(held) = grain() { 0 } else { held.get() } + held.get();
    //                                                           ^ d: tests/rebinds.rs:5
}

pub fn if_let_statement_then_outer(held: Caps) {
    if let Some(held) = grain() { held.get(); } held.get();
    //                                               ^ d: tests/rebinds.rs:5
}

pub fn if_let_else_if_let(held: Caps) {
    let n = if let Some(held) = caps().ok() { 0 } else if let Some(held) = grain() { held.get() } else { 0 };
    //                                                                                    ^ d: tests/rebinds.rs:14
}

pub fn if_let_then_used(held: Caps) {
    if let Some(held) = grain() { drop(&held); held.get(); }
    //                                              ^ d: tests/rebinds.rs:14
}

pub fn if_let_then_named(held: Caps) {
    if let Some(held) = grain() { println!("held"); held.get(); }
    //                                                   ^ d: tests/rebinds.rs:14
}

pub fn if_let_else_used(held: Caps) {
    if let Some(held) = grain() { drop(&held); } else { drop(&held); held.get(); }
    //                                                                    ^ d: tests/rebinds.rs:5
}

pub struct Sack {
    grains: Vec<Grain>,
}

impl Sack {
    pub fn first(&self) -> Option<Grain> {
        None
    }
    pub fn raw(&self) -> Result<Grain, ()> {
        Ok(Grain)
    }
}

pub fn let_else_wrapped_scrutinee(held: Caps, sack: Sack) {
    let Some(held) = sack
        .first()
    else {
        return;
    };
    held.get();
    //   ^ d: tests/rebinds.rs:14
}

pub fn let_else_own_line_one(held: Caps) {
    let Some(held) = grain()
    else { return };
    held.get();
    //   ^ d: tests/rebinds.rs:14
}

pub fn let_else_own_line_wrong_wrapper(held: Caps, sack: Sack) {
    let Some(held) = sack.raw()
    else {
        return;
    };
    held.get();
    //   ^ d: !jump
}

pub fn let_else_closure_scrutinee(held: Caps, sack: Sack) {
    let Some(held) = sack.first().or_else(|| {
        None
    })
    else {
        return;
    };
    held.get();
    //   ^ d: !jump
}

pub fn for_chain_array(held: Caps, v: [Grain; 2]) {
    for held in v.into_iter().rev().skip(1) {
        held.get();
        //   ^ d: tests/rebinds.rs:14
    }
}

pub fn closure_map(held: Caps, v: Vec<Grain>) {
    let n: Vec<usize> = v.iter().take(2).map(|held| held.get()).collect();
    //                                                   ^ d: tests/rebinds.rs:14
}

pub fn for_chain_mapped(held: Caps, v: Vec<Grain>) {
    for held in v.iter().map(|g| Caps) {
        held.get();
        //   ^ d: !jump
    }
}

pub fn closure_enumerated(held: Caps, v: Vec<Grain>) {
    v.iter().enumerate().for_each(|held| {
        held.get();
        //   ^ d: !jump
    });
}

pub fn closure_fold(held: Caps, v: Vec<Grain>) {
    v.iter().fold(0, |held, g| {
        held.get();
        //   ^ d: !jump
        0
    });
}

pub fn closure_second_input(held: Caps, v: Vec<Grain>, w: Vec<Caps>) {
    v.iter().zip(w.iter()).for_each(|held| {
        held.get();
        //   ^ d: !jump
    });
}

pub trait Racked {
    fn iter(&self) -> std::vec::IntoIter<Caps>;
}

impl Racked for std::collections::VecDeque<Grain> {
    fn iter(&self) -> std::vec::IntoIter<Caps> {
        vec![Caps].into_iter()
    }
}

pub fn for_chain_shelved(held: Caps, v: std::collections::VecDeque<Grain>) {
    for held in v.iter().filter(|g| true) {
        held.get();
        //   ^ d: !jump
    }
}

pub fn closure_shelved(held: Caps, v: std::collections::VecDeque<Grain>) {
    v.iter().for_each(|held| {
        held.get();
        //   ^ d: !jump
    });
}

pub struct Bin;
pub struct BinIter;

impl Bin {
    pub fn iter(&self) -> BinIter {
        BinIter
    }
}

impl Iterator for BinIter {
    type Item = Grain;
    fn next(&mut self) -> Option<Grain> {
        None
    }
}

pub fn for_project_iter(held: Caps, bin: &Bin) {
    for held in bin.iter() {
        held.get();
        //   ^ d: tests/rebinds.rs:14
    }
}

pub fn for_project_iter_let(held: Caps) {
    let bin = Bin;
    for held in bin.iter() {
        held.get();
        //   ^ d: tests/rebinds.rs:14
    }
}
