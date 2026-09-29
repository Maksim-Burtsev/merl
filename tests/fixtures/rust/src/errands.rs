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
