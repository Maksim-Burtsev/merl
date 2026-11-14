import { Stylus } from "./quoted";

export const a = (s: Stylus) => <p>You're holding {s.blot()}</p>;
//                                                   ^ d: shop/quoted.ts:2
export const b = (s: Stylus) => <p>It`s {s.blot()}</p>;
export const c = (s: Stylus) => <p title="s.blot()">{s.blot()}</p>;
//                                           ^ d: picker shop/quoted.ts:2, shop/quoted.ts:8
//                                                     ^ d: shop/quoted.ts:2
export const d = (s: Stylus) => s.blot();
//                                ^ d: shop/quoted.ts:2
