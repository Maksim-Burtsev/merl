import pourOver from "./filter.mjs";
import { Cezve, Moka, Ibrik, Jezve, Briki, Fs, Dallah, Lid, Saucer } from "./cezve.js";

export function order() {
  const c = new Cezve();
  const m = new Moka();
  const i = new Ibrik();
  return pourOver(c.strength) + c.foam + m.foam() + i.crema;
  //     ^ d: coffee/filter.mjs:1
  //                ^ d: coffee/cezve.js:4
  //                              ^ d: coffee/cezve.js:2
  //                                       ^ d: coffee/cezve.js:21
  //                                                  ^ d: coffee/cezve.js:27
}

export function lidded() {
  return new Lid().get;
  //               ^ d: coffee/cezve.js:74
}

export function warm() {
  return new Saucer().warmth;
  //                  ^ d: coffee/cezve.js:90
}

export function special() {
  const j = new Jezve();
  const b = new Briki();
  const f = new Fs();
  const d = new Dallah();
  return j.crema() + b.crema + f.stat() + d.decant;
  //       ^ d: coffee/cezve.js:37
  //                   ^ d: coffee/cezve.js:27
  //                             ^ d: coffee/cezve.js:47
  //                                        ^ d: picker coffee/cezve.js:66, coffee/cezve.js:70
}
