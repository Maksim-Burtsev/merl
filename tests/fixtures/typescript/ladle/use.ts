import { Cup } from "./cup";
import { Mug } from "./mug";

const cups: Cup[] = [];
const more: Array<Cup> = [];
const loose = [];
cups.map(c => c.slosh());
//              ^ d: !jump; want ladle/cup.ts:2 (#1488)
cups.forEach(function (c) { c.slosh(); });
//                            ^ d: !jump; want ladle/cup.ts:2 (#1488)
more.map(c => c.slosh());
//              ^ d: !jump; want ladle/cup.ts:2 (#1488)
cups.sort((a, b) => a.heft() - b.heft());
//                               ^ d: !jump; want ladle/cup.ts:3 (#1488)
function each(f: (c: Cup) => void) {}
each(c => c.slosh());
//          ^ d: !jump; want ladle/cup.ts:2 (#1488)
function outer(c: Mug) { return cups.map(c => c.slosh()); }
//                                              ^ d: !jump; want ladle/cup.ts:2 (#1488)
const run = (c: Cup) => c.slosh();
//                        ^ d: ladle/cup.ts:2
loose.map(t => t.slosh());
//               ^ d: !jump
