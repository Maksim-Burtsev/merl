import { Winnow } from "./barn";

Winnow.prototype.sieveChaff = function (id) {
  return id;
};

export function bare() {
  function Winnow() {}
  const r = new Winnow();
  return r.sieveChaff(2);
  //       ^ d: !jump
}

export function own() {
  function Winnow() {
    this.sieveChaff = function (id) {
      return id;
    };
  }
  const r = new Winnow();
  return r.sieveChaff(3);
  //       ^ d: winnow/repatched.js:16
}
