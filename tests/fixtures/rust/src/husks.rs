/* a /* nested */
pub fn husk_nested() {}
pub struct HuskShell;
pub fn husk_real() {}
*/

/** /* nested */
pub fn husk_doc() {}
*/

pub fn husk_real() {}

pub fn husk_use() {
    husk_nested();
//  ^ d: none
    let _ = HuskShell;
//          ^ d: none
    husk_doc();
//  ^ d: none
    husk_real();
//  ^ d: src/husks.rs:11
}
