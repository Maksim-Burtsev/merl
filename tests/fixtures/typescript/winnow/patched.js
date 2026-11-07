import { Winnow } from "./barn";

Winnow.prototype.extra = function () {};

export function local() {
  function Winnow() {
    this.sieveChaff = function (id) {
      return id;
    };
  }
  const r = new Winnow();
  return r.sieveChaff(1);
  //       ^ d: winnow/patched.js:7
}
