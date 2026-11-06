#[derive(Debug, Clone, Copy)]
pub struct Caps;
pub struct Grain;

impl Caps {
    pub fn get(&self) -> usize {
        0
    }
    pub fn grind(self) -> Grain {
        Grain
    }
}

impl Grain {
    pub fn get(&self) -> usize {
        1
    }
}

macro_rules! sift {
    ($x:ident) => {
        let $x = Grain;
    };
}

fn grain() -> Option<Grain> {
    Some(Grain)
}

pub fn lines(held: Caps) -> usize {
    let held = held.grind(); held.get()
    //              ^ d: tests/rebound.rs:9
    //                            ^ d: picker tests/rebound.rs:15, …
}

pub fn typed() -> usize {
    let held: Caps = Caps;
    let held: Grain = Grain; held.get()
    //                            ^ d: picker tests/rebound.rs:15, …
}

pub fn otherwise(held: Caps) -> usize {
    let Some(held) = grain() else { return 0 }; held.get()
    //                                               ^ d: picker tests/rebound.rs:15, …
}

pub fn arms(held: Caps) -> usize {
    match grain() { Some(held) => held.get(), None => held.get() }
    //                                 ^ d: picker tests/rebound.rs:15, …
    //                                                      ^ d: tests/rebound.rs:6
}

pub fn blocks(held: Caps) -> usize {
    { let held = Grain; held.get(); }
    //                       ^ d: picker tests/rebound.rs:15, …
    let n = { let held = Grain; held.get() };
    //                               ^ d: picker tests/rebound.rs:15, …
    { let held = Grain; } held.get()
    //                         ^ d: tests/rebound.rs:6
}

pub fn macros(held: Caps) -> usize {
    sift!(held);
    held.get()
    //   ^ d: picker tests/rebound.rs:15, …
}

pub fn macro_line(held: Caps) -> usize {
    sift!(held); held.get()
    //                ^ d: picker tests/rebound.rs:15, …
}

pub fn shadowed_after(held: Caps) -> usize {
    held.get(); let held = Grain;
    held.get()
    //   ^ d: picker tests/rebound.rs:15, …
    // ^ d: tests/rebound.rs:74
}

pub fn shadowed_inside(held: Caps, c: bool) -> usize {
    if c {
        held.get(); let held = Grain;
    } else {
        held.get(); let held = Grain;
    }
    held.get()
    //   ^ d: tests/rebound.rs:6
}

pub fn printed(held: Caps) -> usize {
    println!("{}", 1);
    assert!(held.get() < 2);
    dbg!(held);
    held.get()
    //   ^ d: tests/rebound.rs:6
}

pub struct Wrap {
    pub held: Caps,
}

pub struct Pair {
    pub held: Grain,
}

pub fn controls(held: Caps, mut pair: Pair) -> usize {
    let n = 1; /* let held = Grain; */ held.get();
    //                                      ^ d: tests/rebound.rs:6
    let s = "let held = Grain;"; held.get();
    //                                ^ d: tests/rebound.rs:6
    let same = held.get() == held.get(); held.get();
    //                                        ^ d: tests/rebound.rs:6
    let w = Wrap { held: Caps }; held.get();
    //                                ^ d: tests/rebound.rs:6
    let v = Wrap { held }; held.get();
    //                          ^ d: tests/rebound.rs:6
    pair.held = Grain; held.get();
    //                      ^ d: tests/rebound.rs:6
    { let held = Grain; held.get(); } held.get();
    //                                     ^ d: tests/rebound.rs:6
    held.get(); let held = Grain;
    //   ^ d: tests/rebound.rs:6
    let _ = (n, s, same, w.held, v.held, pair.held, held);
    0
}
