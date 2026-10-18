"use strict";

function Grinder(burr) {
  this.burr = burr;
}

Grinder.prototype.grind = function (beans) {
  const viaArrow = () => this.burr;
  //                          ^ d: coffee/grinder.cjs:4
  const viaFunction = function () {
    return this.burr;
    //          ^ d: none
  };
  return beans * viaArrow() + viaFunction();
};

Grinder.prototype.coarse = function () {
  return this.grind(3) + this.burr;
  //          ^ d: coffee/grinder.cjs:7
  //                          ^ d: coffee/grinder.cjs:4
};

module.exports = Grinder;
//               ^ d: coffee/grinder.cjs:3
