// An arrow's parameter binds its name in the arrow's body, on the arrow's own line too, over a
// module-level namesake (#531).
const crate = 1;
export const doubled = (crates: number[]) => crates.map(crate => crate * 2);
//                                                               ^ d: shop/doubles.ts:4
//                                                               status: crate: local
export const tripled = (crates: number[]) => crates.map((crate: number) => crate * 3);
//                                                                         ^ d: shop/doubles.ts:7
export const summed = (crates: number[]) => crates.reduce((sum, crate) => sum + crate, crate);
//                                                                              ^ d: shop/doubles.ts:9
//                                                                                     ^ d: shop/doubles.ts:3
export const counted = (crates: number[]) => crates.filter((crate) => crate > 0).length + crate;
//                                                                                        ^ d: shop/doubles.ts:3
export const keyed = (crates: number[]) => crates.map((n) => ({ crate: (x: number) => crate + x + n }));
//                                                                                     ^ d: shop/doubles.ts:3
export const typed = (crates: number[]) => crates.map((crate): number => crate + 1);
//                                                                       ^ d: shop/doubles.ts:16
// A parameter list in a type is no arrow's: the name used beside it is the module's, after an
// annotation, a return type and a type argument.
declare function use(n: number): void;
declare const lucky: boolean;
export class Panel {
  onSave: (crate: number) => void = () => use(crate);
//                                            ^ d: shop/doubles.ts:3
}
export const onPick: (crate: number) => number = (x) => x + crate;
//                                                          ^ d: shop/doubles.ts:3
export function saver(): (crate: number) => void { return () => use(crate); }
//                                                                  ^ d: shop/doubles.ts:3
export const savers = new Array<(crate: number) => void>(crate);
//                                                       ^ d: shop/doubles.ts:3
// An arrow that is one branch of a ternary ends at its `:`; a ternary inside the body is the body's.
export const picked = lucky ? (crate: number) => crate : crate;
//                                                       ^ d: shop/doubles.ts:3
export const chosen = (crates: number[]) => crates.map(crate => lucky ? 0 : crate);
//                                                                          ^ d: shop/doubles.ts:35
