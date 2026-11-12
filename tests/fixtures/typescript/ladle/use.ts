import { Cup } from "./cup";
import { Mug } from "./mug";

const cups: Cup[] = [];
const more: Array<Cup> = [];
const loose = [];
cups.map(c => c.slosh());
//              ^ d: ladle/cup.ts:2
cups.forEach(function (c) { c.slosh(); });
//                            ^ d: ladle/cup.ts:2
more.map(c => c.slosh());
//              ^ d: ladle/cup.ts:2
cups.sort((a, b) => a.heft() - b.heft());
//                               ^ d: ladle/cup.ts:3
function each(f: (c: Cup) => void) {}
each(c => c.slosh());
//          ^ d: ladle/cup.ts:2
function outer(c: Mug) { return cups.map(c => c.slosh()); }
//                                              ^ d: ladle/cup.ts:2
const run = (c: Cup) => c.slosh();
//                        ^ d: ladle/cup.ts:2
loose.map(t => t.slosh());
//               ^ d: !jump
const mugs: Mug[] = [];
mugs.filter(c => c.slosh());
//                 ^ d: ladle/mug.ts:2
cups.map((c, i) => i.slosh());
//                   ^ d: !jump
cups.forEach(c => c.slosh()); mugs.forEach(c => c.heft());
//                  ^ d: ladle/cup.ts:2
cups.forEach(c => c.slosh()); mugs.forEach(c => c.heft());
//                                                ^ d: ladle/mug.ts:3
cups.forEach((c) => c.slosh()); mugs.forEach((c) => c.heft());
//                    ^ d: ladle/cup.ts:2
cups.map((c) => c.jars.map(c => c.slosh()));
//                                ^ d: !jump
cups.map(c => mugs.reduce((n, c) => n + c.heft(), 0));
//                                        ^ d: !jump
mugs.forEach(c => cups.forEach(d => c.slosh()));
//                                    ^ d: ladle/mug.ts:2
cups.forEach((c) => c.slosh()); mugs.forEach((c) => {
  c.heft();
//  ^ d: !jump
});
cups.forEach(c => c.slosh()); mugs.forEach(c => {
  c.heft();
//  ^ d: ladle/mug.ts:3
});
const o = { c: 1, k: 2 };
cups.forEach(c => { const { c: n } = o; c.slosh(); });
//                                        ^ d: ladle/cup.ts:2
cups.forEach(c => { const { k: c } = o; c.slosh(); });
//                                        ^ d: !jump
cups.forEach(c => { const { c } = o; c.slosh(); });
//                                     ^ d: !jump
cups.forEach(c => { for (const [k, v] of Object.entries(c)) c.slosh(); });
//                                                            ^ d: ladle/cup.ts:2
