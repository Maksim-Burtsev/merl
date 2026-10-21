//! The third adversarial corner of #1126 for Rust, its answers rust-analyzer's.

pub mod leaves;
pub mod pantry;

use self::pantry::shelf::*;
use self::pantry::{Infuser, Sheet};
#[cfg(unix)]
use self::pantry::north::kind;
#[cfg(not(unix))]
use self::pantry::south::kind;

fn gulp() -> u32 {
    20
}

pub trait Sniff {
    fn sniff(&self) -> usize;
}

impl Sniff for str {
    fn sniff(&self) -> usize {
        self.len()
        //   ^ d: none
    }
}

pub fn std_values(leaf: &Leaf) -> usize {
    let name = String::from("oolong");
    name.len() + leaf.name.len() + leaf.len()
    //   ^ d: none
    //                     ^ d: none
    //                                  ^ d: src/infusion/leaves.rs:13
}

pub fn std_chain(leaf: &Leaf) -> usize {
    let mut cup = Vec::new();
    cup.push(leaf.trim().len());
    //  ^ d: none
    //            ^ d: src/infusion/leaves.rs:17
    cup.len()
}

pub fn shadow_on_own_line(leaf: &Leaf) -> usize {
    let leaf = leaf.trim();
    //         ^ d: src/infusion.rs:44
    //              ^ d: src/infusion/leaves.rs:17
    leaf.len()
    //   ^ d: none
}

pub fn builder() -> u32 {
    Infuser::start()
        .leaves(3)
//       ^ d: src/infusion/leaves.rs:60
        .water(2)
//       ^ d: src/infusion/leaves.rs:65
        .finish()
//       ^ d: src/infusion/leaves.rs:70
        .taste()
//       ^ d: src/infusion/leaves.rs:80
}

pub fn on_a_result() -> u32 {
    Leaf::new(2).minutes() + Leaf::new(3).alarm() + Sheet::new(4).label().len() as u32
    //           ^ d: src/infusion/leaves.rs:27
    //                                    ^ d: src/infusion/leaves.rs:37
    //                                              ^ d: src/infusion/leaves.rs:1
    //                                                            ^ d: src/infusion/leaves.rs:43
}

pub fn overridden() -> usize {
    let sheet = Sheet::new(5);
    Leaf::new(4).label().len() + sheet.label().len() + sheet.grade as usize
    //           ^ d: src/infusion/leaves.rs:43
    //                                 ^ d: src/infusion/leaves.rs:43
    //                                                       ^ d: src/infusion/leaves.rs:2
}

pub fn bound<T: Timed>(t: &T) -> u32 {
    t.minutes() + t.alarm()
    //              ^ d: src/infusion/leaves.rs:37
}

pub fn extension() -> usize {
    "oolong".sniff()
    //       ^ d: src/infusion.rs:22
}

pub fn loop_header(leaves: Vec<Leaf>) -> u32 {
    let mut total = 0;
    for (grade, leaf) in leaves.iter().enumerate() {
        total += leaf.grade + grade as u32;
        //            ^ d: src/infusion/leaves.rs:2
        //                    ^ d: src/infusion.rs:92
    }
    total
}

pub fn destructured(leaf: Leaf) -> usize {
    let Leaf { grade, name } = leaf;
    name.len() + grade as usize
    //           ^ d: src/infusion.rs:101
}

pub fn alias(t: &Tin) -> u32 {
    let tin = Tin::new(1);
    //             ^ d: src/infusion/leaves.rs:9
    tin.label().len() as u32 + t.label().len() as u32
    //  ^ d: src/infusion/leaves.rs:43
    //                           ^ d: src/infusion/leaves.rs:43
}

pub fn imports() -> u32 {
    sip() + gulp() + kind().len() as u32
//  ^ d: src/infusion/leaves.rs:85
    //      ^ d: src/infusion.rs:13
    //               ^ d: picker src/infusion/pantry.rs:9, src/infusion/pantry.rs:15
}

pub fn nested() -> u32 {
    fn gulp() -> u32 {
        sip() + 3
//      ^ d: src/infusion/leaves.rs:85
    }
    gulp()
//  ^ d: src/infusion.rs:122
}

pub fn closure_param() -> u32 {
    let sip = |gulp: u32| gulp + 1;
    sip(gulp())
//  ^ d: src/infusion.rs:131
    //  ^ d: src/infusion.rs:13
}

pub fn block_scope() -> u32 {
    let cup = 1;
    let inner = {
        let cup = cup + 1;
        //        ^ d: src/infusion.rs:138
        cup
//      ^ d: src/infusion.rs:140
    };
    inner + cup
    //      ^ d: src/infusion.rs:138
}

pub fn captured(steeper: &dyn Steeper) -> String {
    let gulp = 7;
    format!("{gulp} {}", steeper.label())
    //        ^ d: src/infusion.rs:150
    //                           ^ d: src/infusion/leaves.rs:31
}

impl Infuser {
    pub fn again(self) -> Self {
        Self::start().leaves(Leaf::MAX)
        //    ^ d: src/infusion/leaves.rs:56
        //                         ^ d: src/infusion/leaves.rs:7
    }
}

pub const NOTE: &str = "
pub fn gulp() -> u32 { 0 }
impl Infuser { pub fn finish(self) {} }
";

/* pub fn sip() -> u32 { 1 }
   pub struct Liquor; */

pub fn alias_paths(t: &Tin) -> usize {
    Tin::label(t).len() + Kettle::new().len()
    //   ^ d: src/infusion/leaves.rs:43
    //                            ^ d: !jump
}

pub fn extension_bound() -> usize {
    let s = "oolong";
    s.sniff() + "oolong".len()
    //^ d: src/infusion.rs:22
}

pub fn shadow_back_to_leaf(leaf: Option<Leaf>) -> usize {
    let leaf = leaf.unwrap();
    leaf.len()
    //   ^ d: src/infusion/leaves.rs:13
}

pub fn label_of(leaf: &Leaf) -> usize {
    let label = leaf.label();
    label.len()
    //    ^ d: none
}

pub fn std_tuple(leaf: &mut Leaf) -> usize {
    let (mut cup, mut pot, kept) = (vec![], vec!['x'], Leaf::new(1));
    cup.push(leaf.len());
    //  ^ d: none
    let mut jar = vec![1];
    jar.push(2);
    //  ^ d: none
    pot.len() + kept.len() + cup.len()
    //  ^ d: none
    //               ^ d: src/infusion/leaves.rs:13
}

pub fn std_array(leaves: Vec<Leaf>) -> usize {
    let n = [1, 2].len() + leaves[0].len();
    //             ^ d: none
    //                               ^ d: src/infusion/leaves.rs:13
    [Leaf::new(1), Leaf::new(2)]
        .len()
    //   ^ d: none
        + n
}

pub fn std_literal_then_more() -> usize {
    let w = vec![Leaf::new(1)].remove(0);
    w.len() + [Leaf::new(2)].into_iter().next().unwrap().len()
    //^ d: src/infusion/leaves.rs:13
    //                                                   ^ d: src/infusion/leaves.rs:13
}
