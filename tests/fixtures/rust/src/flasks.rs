mod cuvette;
mod phial;
mod retort;

use cuvette::Cuvette;
use cuvette::Cuvette as Beaker;
use phial::Phial;

// Types imported through a module this file declares (`mod cuvette;`, `use cuvette::Cuvette;`):
// `Type::new()` and `Type::MAX` land on that type's own, never on `Retort::new(&self)` (#1489).
pub fn pour() -> u32 {
    let _ = Cuvette::new();
    //               ^ d: src/flasks/cuvette.rs:5
    let _ = Phial::new();
    //             ^ d: src/flasks/phial.rs:5
    let _ = cuvette::Cuvette::new();
    //                        ^ d: src/flasks/cuvette.rs:5
    let _ = Beaker::new();
    //              ^ d: src/flasks/cuvette.rs:5
    Cuvette::MAX + Phial::MAX
    //       ^ d: src/flasks/cuvette.rs:4
    //                    ^ d: src/flasks/phial.rs:4
}
