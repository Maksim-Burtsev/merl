import { Winnow } from "./barn";

export function inner() {
  const r = new Winnow();
  return new Winnow().sieveChaff(8);
  //         ^ d: winnow/barn.ts:1
  function nested() {
    function Winnow() {}
    return Winnow;
  }
}

export function blocked() {
  if (inner) {
    function Winnow() {}
  }
  return new Winnow().sieveChaff(9);
  //         ^ d: winnow/barn.ts:1
}

export function rebound() {
  let Winnow = inner;
  return Winnow;
  //     ^ d: winnow/gleaned.js:22
  function inner() {}
}

export function param(Winnow) {
  return Winnow;
  //     ^ d: winnow/gleaned.js:31
  function Winnow() {}
}

export function inside() {
  if (inner) {
    const Winnow = inner;
    return Winnow;
    //     ^ d: winnow/gleaned.js:36
  }
  function Winnow() {}
}
