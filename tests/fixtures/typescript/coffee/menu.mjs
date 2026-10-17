import pourOver from "./filter.mjs";
import { Cezve, Moka, Ibrik } from "./cezve.js";

export function order() {
  const c = new Cezve();
  const m = new Moka();
  const i = new Ibrik();
  return pourOver(c.strength) + c.foam + m.foam() + i.crema;
  //     ^ d: coffee/filter.mjs:1
  //                ^ d: coffee/cezve.js:4
  //                              ^ d: coffee/cezve.js:2
  //                                       ^ d: coffee/cezve.js:21
  //                                                  ^ d: coffee/cezve.js:29; want coffee/cezve.js:27 (#1004)
}
