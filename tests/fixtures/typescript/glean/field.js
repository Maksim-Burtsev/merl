import { Sheaf } from "./barn";

export function harvest(ripe) {
  if (ripe) {
    function Sheaf() {}
    return new Sheaf();
    //         ^ d: glean/field.js:5
  }
  return new Sheaf();
  //         ^ d: glean/barn.ts:1
}

export function stacked(ripe) {
  if (ripe) {
    function Sheaf() {}
  }
  const s = new Sheaf();
  //            ^ d: glean/barn.ts:1
  return s.bind(1);
  //       ^ d: glean/barn.ts:2
}

export function hoisted() {
  function Sheaf() {}
  return new Sheaf();
  //         ^ d: glean/field.js:24
}

export function built(ripe) {
  if (ripe) {
    function Sheaf() {
      this.bind = function (n) {
        return n;
      };
    }
  }
  const s = new Sheaf();
  return s.bind(2);
  //       ^ d: glean/barn.ts:2
}

if (globalThis.ripe) {
  function Sheaf() {
    this.bind = function (n) {
      return n;
    };
  }
}

export function top() {
  return new Sheaf().bind(3);
  //         ^ d: glean/barn.ts:1
  //                 ^ d: glean/barn.ts:2
}
