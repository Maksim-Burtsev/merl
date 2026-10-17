class Pot {}
function scale(o) {
  return o;
}

export function dose(width, Pot) {
  const r = width
    * scale({ Pot }).x;
  //          ^ d: coffee/dosing.js:6
  /** @type {Object} */ const opts = { Pot };
  //                                   ^ d: coffee/dosing.js:6
  return r + opts.Pot;
}

const timeout = 5;
/*
 * Waits {timeout} seconds.
//        ^ d: coffee/dosing.js:15
 */
export const wait = timeout + new Pot();
