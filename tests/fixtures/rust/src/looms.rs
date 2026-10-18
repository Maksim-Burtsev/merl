//! #1045: a method called on the result of another call, on one line and one call per line.

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
