import { Sheaf } from "./barn";

export function before(ripe) {
  const s = new Sheaf();
  //            ^ d: glean/barn.ts:1
  if (ripe) {
    function Sheaf() {
      this.bind = function (n) {
        return n;
      };
    }
  }
  return s.bind(1);
  //       ^ d: glean/barn.ts:2
}

if (globalThis.ripe) {
  function Sheaf() {}
} else {
  new Sheaf();
  //  ^ d: glean/barn.ts:1
}

new Sheaf();
//  ^ d: glean/barn.ts:1

for (const x of []) {
  function Sheaf() {}
}

export const late = () => new Sheaf().bind(4);
//                            ^ d: glean/barn.ts:1
//                                    ^ d: glean/barn.ts:2

export function braced(ripe) {
  if (ripe) function Sheaf() {}
  return new Sheaf();
  //         ^ d: glean/barn.ts:1
}

export function nested(ripe) {
  if (ripe) {
    function Sheaf() {}
    const inner = () => new Sheaf();
    //                      ^ d: glean/probe.js:43
    return inner;
  }
  try {
    function Sheaf() {}
  } catch (e) {
    return new Sheaf();
    //         ^ d: glean/barn.ts:1
  }
  return null;
}

export function inline(ripe) {
  if (ripe) { function Sheaf() { this.bind = (n) => n; } }
  return new Sheaf().bind(5);
  //         ^ d: glean/barn.ts:1
  //                 ^ d: glean/barn.ts:2
}

export function patched(ripe) {
  if (ripe) {
    function Sheaf() {}
    Sheaf.prototype.bind = function (n) {
      return n;
    };
  }
  const s = new Sheaf();
  return s.bind(6);
  //       ^ d: glean/barn.ts:2
}
