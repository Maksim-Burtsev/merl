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

// A path that starts at a module nested in an inline `mod` of this file is that module's, never
// the file-level `mod cuvette;` of the same name: a picker, as on master (#1610).
pub mod stand {
    mod cuvette {
        pub fn swirl() -> u32 {
            2
        }
    }
    pub fn run() -> u32 {
        cuvette::swirl()
        //       ^ d: picker src/flasks.rs:29, src/flasks/cuvette.rs:10
    }
}
