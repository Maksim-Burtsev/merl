//! #353: parameters, `let`, `if let`, closures and `for` bind their names in their block.

pub fn config() -> u32 {
    0
}

pub struct Printer {
    hyperlink: u32,
}

impl Printer {
    pub fn hyperlink(&mut self, config: u32) -> &mut Printer {
        self.hyperlink = config;
        //               ^ d: src/locals.rs:12
        //               status: config → Printer::hyperlink::config (local)
        self
    }

    pub fn wrapped(
        &mut self,
        config: u32,
        mut widths: u32,
    ) -> u32 {
        widths += config;
        // ^ d: src/locals.rs:22
        //        ^ d: src/locals.rs:21
        //        status: config → Printer::wrapped::config (local)
        widths
    }
}

pub fn or(v: u32) -> u32 {
    v
}

pub fn pair() -> u32 {
    let (or, and) = (1, 2);
    or + and
//  ^ d: src/locals.rs:37
//  status: or → pair::or (local)
}

pub fn err() -> u32 {
    0
}

pub fn check(kind: u32) -> Result<(), u32> {
    let err = |k: u32| k + 1;
    Err(err(kind))
    //  ^ d: src/locals.rs:48
    //      ^ d: src/locals.rs:47
}

pub fn trimmed(stem: &str) -> usize {
    let stem = stem.trim();
    //  ^ d: src/locals.rs:55
    //         ^ d: src/locals.rs:54
    let stem = stem.len();
    //         ^ d: src/locals.rs:55
    stem
    // ^ d: src/locals.rs:58
}

pub fn maybe(v: Option<u32>) -> u32 {
    if let Some(n) = v {
        or(n)
//      ^ d: src/locals.rs:32
        // ^ d: src/locals.rs:65
    } else {
        v.map_or(0, or)
        //          ^ d: src/locals.rs:32
    }
}

pub fn sum_all(pairs: &[(u32, u32)]) -> u32 {
    let mut total = 0;
    for (_, w) in pairs {
        total += w;
        //       ^ d: src/locals.rs:77
        // ^ d: src/locals.rs:76
    }
    total
}

pub fn arms(v: Option<u32>, kind: u32) -> u32 {
    match v {
        Some(kind) => {
            kind + 1
            // ^ d: src/locals.rs:87
        }
        None => kind,
        //      ^ d: src/locals.rs:85
    }
}

pub fn branches(v: Option<u32>, n: u32) -> u32 {
    if let Some(n) = v {
        n
    } else {
        n
        // ^ d: src/locals.rs:96
    }
}

pub fn callback(items: Vec<u32>) -> Vec<u32> {
    items
        .into_iter()
        .map(|stem| {
            stem + 1
            // ^ d: src/locals.rs:108
        })
        .collect()
}

pub fn literal(hyperlink: u32) -> Printer {
    Printer { hyperlink: hyperlink + 1 }
    //        ^ d: src/locals.rs:8
    //                   ^ d: src/locals.rs:115
}

pub fn bounded<T>(pairs: T) -> usize
where
    T: IntoIterator,
{
    pairs.into_iter().count()
    // ^ d: src/locals.rs:121
}

pub fn outside(stem: u32) -> u32 {
    fn inner() -> u32 {
        stem()
        // ^ d: src/locals.rs:137
    }
    inner() + stem
}

fn stem() -> u32 {
    0
}

impl Printer {
    pub fn fresh() -> Self {
        Self { hyperlink: 0 }
        //     ^ d: src/locals.rs:8
    }
}

pub fn unpacked(p: Printer) -> u32 {
    let Printer { hyperlink: h } = p;
    //            ^ d: src/locals.rs:8
    h
}
