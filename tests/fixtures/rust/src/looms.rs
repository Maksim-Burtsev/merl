//! #1045: a method called on the result of another call, on one line and one call per line; #1046: through `?`, `.unwrap()` and `.expect(..)`; #1048: a receiver whose type another crate of the workspace declares; #1047: `let x = x(..)`, whose right-hand `x` is the function.

pub struct Loom {
    width: usize,
}

pub struct LoomBuilder {
    width: usize,
}

pub struct Weave<W> {
    out: W,
}

impl LoomBuilder {
    pub fn new() -> LoomBuilder {
        LoomBuilder { width: 0 }
    }

    pub fn width(&mut self, n: usize) -> &mut Self {
        self.width = n;
        self
    }

    pub fn build(&self) -> Loom {
        Loom { width: self.width }
    }

    pub fn weave<W>(&self, out: W) -> Weave<W> {
        Weave { out }
    }
}

impl Loom {
    pub fn shuttle(&self) -> usize {
        self.width
    }
}

impl<W> Weave<W> {
    pub fn shuttle(&self) -> &W {
        &self.out
    }
}

pub struct Bobbin;

impl Bobbin {
    pub fn spin(&self) -> Loom {
        Loom { width: 1 }
    }
}

pub struct Spindle;

impl Spindle {
    pub fn spin(&self) -> Weave<usize> {
        Weave { out: 2 }
    }
}

pub fn woven(b: Bobbin) -> usize {
    LoomBuilder::new().width(2).build().shuttle();
    //                 ^ d: src/looms.rs:20
    //                          ^ d: src/looms.rs:25
    //                                  ^ d: src/looms.rs:35
    let loom = LoomBuilder::new()
        .width(3)
        .build();
    loom.shuttle();
    //   ^ d: src/looms.rs:35
    LoomBuilder::new()
        .width(4)
        //^ d: src/looms.rs:20
        .weave(5)
        .shuttle();
        //^ d: src/looms.rs:41
    b.spin().shuttle()
    //       ^ d: src/looms.rs:35
}

pub fn unknown(x: Unknown) -> usize {
    x.spin().shuttle()
    //       ^ d: picker src/looms.rs:35, src/looms.rs:41
}

pub fn knot() -> Result<Loom, String> {
    Ok(Loom { width: 6 })
}

pub fn maybe() -> Option<Bobbin> {
    Some(Bobbin)
}

pub fn tied() -> Result<usize, String> {
    let a = knot()?;
    a.shuttle();
    //^ d: src/looms.rs:35
    let b = maybe().unwrap();
    b.spin();
    //^ d: src/looms.rs:49
    let c = maybe().expect("a bobbin");
    c.spin();
    //^ d: src/looms.rs:49
    knot()?.shuttle();
    //      ^ d: src/looms.rs:35
    Ok(maybe().unwrap().spin().shuttle())
    //                         ^ d: src/looms.rs:35
}

pub fn twine(n: usize) -> Loom {
    Loom { width: n }
}

pub fn twined() -> usize {
    let twine = twine(1);
    //          ^ d: src/looms.rs:111
    twine.shuttle()
    //    ^ d: src/looms.rs:35
}

pub fn twisted() -> usize {
    let twine = twine(2).shuttle();
    //          ^ d: src/looms.rs:111
    let knot = knot().unwrap();
    //         ^ d: src/looms.rs:87
    knot.shuttle() + twine
    //   ^ d: src/looms.rs:35
}

impl Bobbin {
    pub fn twine(&self) -> usize {
        0
    }
}

use other::Reel;

pub struct Winch;

impl Winch {
    pub fn wind(&self) -> u32 {
        2
    }
}

pub fn reeled(reel: &Reel) -> u32 {
    reel.wind()
    //   ^ d: other/src/lib.rs:24
}

#[cfg(test)]
mod spooled {
    use other::Spool;

    fn wound() -> u32 {
        Spool::new().wind()
        //           ^ d: other/src/lib.rs:36
    }

    mod deeper {
        pub struct Spool;

        impl Spool {
            pub fn new() -> Spool {
                Spool
            }
        }

        fn wound() {
            Spool::new().wind();
            //           ^ d: !jump
        }
    }
}

pub fn rewoven() -> usize {
    LoomBuilder::new()
        .weave(
            7,
        )
        .shuttle();
        //^ d: src/looms.rs:41
    let held = (
        LoomBuilder::new(),
    );
    held.0.build().shuttle()
    //             ^ d: picker src/looms.rs:35, src/looms.rs:41
}
