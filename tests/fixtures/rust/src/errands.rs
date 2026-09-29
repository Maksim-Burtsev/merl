//! #363: a bare name is the item this file declares where the cursor sees it.

fn helper() -> u32 {
    1
}

pub fn run() -> u32 {
    helper()
    // ^ d: src/errands.rs:3
    // status: helper: in this file
}

pub fn shadow(helper: u32) -> u32 {
    helper + 1
    // ^ d: !jump
}

pub fn nested() -> u32 {
    fn helper() -> u32 {
        3
    }
    helper()
    // ^ d: src/errands.rs:19
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkdirp() -> u32 {
        4
    }

    #[test]
    fn helpers() {
        assert_eq!(mkdirp() + helper(), 5);
        //         ^ d: src/errands.rs:30
        //                    ^ d: src/errands.rs:3
    }
}

pub fn locals() -> u32 {
    let doubled = |helper: u32| helper * 2;
    //                          ^ d: !jump
    let helper = doubled(1);
    helper
    // ^ d: !jump
}

// After the review: every refusal with a namesake in `chores`, so a jump shows.
pub fn bound_let() -> u32 {
    let helper = 4;
    //  ^ d: !jump
    helper + 1
    // ^ d: !jump
}

pub fn bound_for(v: Vec<u32>) -> u32 {
    let mut n = 0;
    for helper in v {
        n += helper;
        //   ^ d: !jump
    }
    n
}

pub fn bound_closure() -> u32 {
    let f = |helper| helper + 1;
    //               ^ d: !jump
    f(1)
}

pub fn bound_arm(v: Option<u32>) -> u32 {
    match v {
        Some(helper) => helper,
        //              ^ d: !jump
        None => 0,
    }
}

pub fn bound_at(v: u32) -> u32 {
    match v {
        helper @ 0..=9 => helper,
        //                ^ d: !jump
        _ => 0,
    }
}

pub fn bound_wrapped(p: (u32, u32)) -> u32 {
    let (
        a,
        helper,
    ) = p;
    a + helper
    //  ^ d: !jump
}

pub fn bound_tuple(v: Vec<(u32, u32)>) -> u32 {
    v.iter().map(|(a, helper)| a + helper).sum()
    //                             ^ d: !jump
}

pub fn bound_pattern((a, helper): (u32, u32)) -> u32 {
    a + helper
    //  ^ d: !jump
}

pub struct Tally;

impl Tally {
    pub fn new() -> Tally {
        Tally
    }
}

pub fn generic<Tally: Default>() -> Tally {
    Tally::default()
    // ^ d: !jump
}

fn tally() -> u32 {
    //^ d: !jump
    2
}

macro_rules! tally {
    () => {
        2
    };
}

pub fn tallies() -> u32 {
    tally!() + tally()
    // ^ d: src/errands.rs:126
    //         ^ d: src/errands.rs:121
}

pub fn too_early() -> u32 {
    late!()
    // ^ d: !jump
}

macro_rules! late {
    () => {
        2
    };
}

pub struct Stall {
    pub helper: u32,
}

pub fn stall() -> Stall {
    Stall { helper: 1 }
    //      ^ d: !jump
}

pub fn imported() -> u32 {
    use crate::chores::tally;
    tally()
    // ^ d: picker src/errands.rs:121, src/errands.rs:126, src/chores.rs:20, src/chores.rs:24; want src/chores.rs:20 (#350)
}

mod apart {
    pub fn call() -> u32 {
        helper()
        // ^ d: !jump
    }
}

fn regex() -> u32 {
    2
}

pub fn escaped() -> String {
    regex::escape("a")
    // ^ d: !jump
}

pub fn blocked() -> u32 {
    {
        fn helper() -> u32 {
            8
        }
        helper()
        // ^ d: !jump
    }
}

pub fn outer() -> u32 {
    fn helper() -> u32 {
        9
    }
    fn inner() -> u32 {
        helper()
        // ^ d: !jump
    }
    inner()
}

mod inner {
    pub struct Tally;

    impl Tally {
        pub fn new() -> Tally {
            Tally
        }
    }

    pub fn make() -> Tally {
        Tally::new()
        //     ^ d: src/errands.rs:205
        //     status: new → inner::Tally::new (via Tally)
    }
}

pub fn weighed() -> u32 {
    Tally::stamp()
    //     ^ d: src/chores.rs:15
}

#[cfg(test)]
mod picked {
    use super::*;
    use crate::chores::tally;

    #[test]
    fn picks() {
        assert_eq!(tally(), 1);
        //         ^ d: picker src/errands.rs:121, src/errands.rs:126, src/chores.rs:20, src/chores.rs:24; want src/chores.rs:20 (#350)
    }
}
