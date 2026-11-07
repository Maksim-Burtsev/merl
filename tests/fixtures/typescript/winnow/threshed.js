import { Winnow } from "./barn";

export function assigned() {
  function Winnow() {
    this.sieveChaff = function (id) {
      return id;
    };
  }
  const r = new Winnow();
  return r.sieveChaff(1) + new Winnow().sieveChaff(2);
  //       ^ d: winnow/threshed.js:5
  //                                    ^ d: winnow/threshed.js:5
}

export function imported() {
  const r = new Winnow();
  return r.sieveChaff(4);
  //       ^ d: winnow/barn.ts:2
}
