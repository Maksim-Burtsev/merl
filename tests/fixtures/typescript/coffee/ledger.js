"use strict";

class Scale {
  weigh(n) {
    return n;
  }
}

function count() {
  const last = 0;
  return last;
}

function tally(beans, cups) {
  let total, count, last;
  let first = beans[0], rest = beans.slice(1, cups), scale = new Scale();
  total = first + rest.length;
//^ d: coffee/ledger.js:15
  //      ^ d: coffee/ledger.js:16
  //              ^ d: coffee/ledger.js:16
  count = cups;
//^ d: coffee/ledger.js:15
  last = rest[0];
//^ d: coffee/ledger.js:15
  return scale.weigh(total + count + last);
  //           ^ d: coffee/ledger.js:4
  //                         ^ d: coffee/ledger.js:15
  //                                 ^ d: coffee/ledger.js:15
}

function weigh(
  beans,
  count,
//^ d: coffee/ledger.js:33
) {
  let total, last;
  //         ^ d: coffee/ledger.js:36
  total = beans + count;
  //              ^ d: coffee/ledger.js:33
  return total + last;
  //             ^ d: coffee/ledger.js:36
}
