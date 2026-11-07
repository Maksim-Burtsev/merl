import { Winnow } from "./barn";

export function prototyped() {
  function Winnow() {}
  Winnow.prototype.sieveChaff = function (id) {
    return id;
  };
  const r = new Winnow();
  return r.sieveChaff(3);
  //       ^ d: winnow/sifted.js:5
}
