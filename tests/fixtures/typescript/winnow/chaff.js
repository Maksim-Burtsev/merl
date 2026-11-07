import { Winnow } from "./barn";

export function prototyped() {
  function Winnow() {}
  Winnow.prototype.sieveChaff = function (id) {
    return id;
  };
  return Winnow;
}

export function bare() {
  function Winnow() {}
  const r = new Winnow();
  return r.sieveChaff(5);
  //       ^ d: !jump
}
