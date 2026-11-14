import { Carafe } from "./carafe";

/** @param {typeof Carafe} k */
export function classed(k) {
  k.decant();
//  ^ d: !jump
}

/** @param {Carafe} c */
export function served(c) {
  c.decant(2);
//  ^ d: statics/carafe.ts:5
}
