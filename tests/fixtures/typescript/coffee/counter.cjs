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
  //^ d: coffee/grinder.cjs:7
  tamp(darken(cups));
//^ d: coffee/counter.cjs:37
  //   ^ d: coffee/roast.cjs:3
  const oven = new Oven(1);
  //               ^ d: coffee/roast.cjs:7
  roast.measure({ origin: "", scoops: oven.level });
  //    ^ d: coffee/roast.cjs:21
  //                                       ^ d: coffee/roast.cjs:9
  var dose = 1;
  if (cups > 2) {
    var dose = 2;
  }
  return dose + recipe.length + roast.darken(dose);
  //     ^ d: picker coffee/counter.cjs:28, coffee/counter.cjs:30
  //                                  ^ d: coffee/roast.cjs:3
}

function tamp(x) {
  return x;
}

module.exports = { serve };
//                 ^ d: coffee/counter.cjs:15
