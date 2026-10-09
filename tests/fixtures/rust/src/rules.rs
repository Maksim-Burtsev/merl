//! #611: the rules of `d` in Rust that only a comment held.

pub struct Lamp {
    pub glow: u32,
}

impl Lamp {
    pub fn glow<T>(&self) -> u32 {
        0
    }
}

pub fn lit(v: Vec<Lamp>) -> u32 {
    v.iter().map(|l| l.glow::<u8>() + l.glow).sum()
    //                 ^ d: src/rules.rs:8
    //                                  ^ d: src/rules.rs:4
}

#[proc_macro_derive(Knob)]
pub fn knob_derive(input: TokenStream) -> TokenStream {
    input
}

#[derive(Knob)]
//       ^ d: src/rules.rs:19
pub struct Winch;

pub struct Pulley {
    pub rope: u32,
}

pub fn hoist(
    rope: u32,
) -> Pulley {
    Pulley {
        rope,
    }
}

pub fn pulled(v: Vec<Pulley>) -> u32 {
    v.iter().map(|p| p.rope).sum()
    //                 ^ d: src/rules.rs:29
}

pub struct Crank(Vec<u32>);

impl Crank {
    pub fn whirl(&self) -> u32 { self.0.whirl() }
    //                                  ^ d: none
}

pub struct Amber;

pub enum Tint {
    Coral,
}

pub struct Wagon;

pub struct Sled;

impl Lamp {
    pub fn polish(&self) -> u32 {
        0
    }
}

pub fn polished(l: &Lamp) -> u32 {
    l.polish()
    //^ d: src/rules.rs:63
}
