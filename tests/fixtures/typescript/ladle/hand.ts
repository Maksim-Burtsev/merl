import { Cup } from "./cup";
import { Mug } from "./mug";

function both(n: number, f: (c: Cup, m: Mug) => void) {}
both(1, (c, m) => m.heft());
//                  ^ d: ladle/mug.ts:3
both(1, (c, m) => c.heft());
//                  ^ d: ladle/cup.ts:3
function every<T>(f: (c: T) => void) {}
every(c => c.heft());
//           ^ d: !jump
function pass(f: (c: Cup) => void) {}
function pour(pass: (c: Mug) => void) { pass(c => c.slosh()); }
//                                                  ^ d: !jump
const box = { pass };
box.pass(c => c.slosh());
//              ^ d: !jump
function lift(f: (c: Cup) => void) {}
const mugs: Mug[] = [];
lift(c => { for (const c of mugs) c.slosh(); });
//                                  ^ d: !jump
lift(c => { const k = c; k.heft(); c.slosh(); });
//                                   ^ d: ladle/cup.ts:2
function each(f: (c: Cup) => void) {}
const pots = { each(f: (m: Mug) => void) {} };
pots.each(c => c.slosh());
//               ^ d: !jump
function stir() {
  each(c => c.slosh());
  //          ^ d: ladle/cup.ts:2
  pots.each(c => c.slosh());
  //               ^ d: !jump
  both(1, (c, m) => m.heft());
  //                  ^ d: ladle/mug.ts:3
}
