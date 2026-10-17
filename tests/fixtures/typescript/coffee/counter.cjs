"use strict";

const Grinder = require("./grinder.cjs");
const { darken, Roaster: Oven } = require("./roast.cjs");
const roast = require("./roast.cjs");

const recipe = `
function tamp() {}
class Grinder {}
var dose = 9;
`;
// function darken() {}
/* function serve() {} */

function serve(cups) {
  const g = new Grinder(2);
  //            ^ d: coffee/grinder.cjs:3
  g.grind(cups);
  //^ d: none; want coffee/grinder.cjs:7 (#1001)
  tamp(darken(cups));
//^ d: coffee/counter.cjs:37
  //   ^ d: none; want coffee/roast.cjs:3 (#1007)
  const oven = new Oven(1);
  //               ^ d: none; want coffee/roast.cjs:7 (#1007)
  roast.measure({ origin: "", scoops: oven.level });
  //    ^ d: coffee/roast.cjs:21
  //                                       ^ d: none; want coffee/roast.cjs:9 (#1001)
  var dose = 1;
  if (cups > 2) {
    var dose = 2;
  }
  return dose + recipe.length + roast.darken(dose);
  //     ^ d: coffee/counter.cjs:28; want picker coffee/counter.cjs:28, coffee/counter.cjs:30 (#1003)
  //                                  ^ d: none; want coffee/roast.cjs:3 (#1007)
}

function tamp(x) {
  return x;
}

module.exports = { serve };
//                 ^ d: coffee/counter.cjs:15
