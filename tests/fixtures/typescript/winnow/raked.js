import { Winnow } from "./barn";

export function below() {
  const r = new Winnow();
  return r.sieveChaff(6) + new Winnow().sieveChaff(7);
  //       ^ d: winnow/raked.js:10
  //                           ^ d: winnow/raked.js:9
  //                                    ^ d: winnow/raked.js:10
  function Winnow() {
    this.sieveChaff = function (id) {
      return id;
    };
  }
}
