"use strict";

const Grinder = require("./grinder.cjs");

function Mill(stones) {
  this.stones = stones;
  function turn() {
    return 1;
  }
  this.turn = turn;
}

Mill.prototype.grind = async function () {
  return this.stones;
  //          ^ d: coffee/mill.cjs:6
};

Mill.prototype.fast = () => this;

class FineGrinder extends Grinder {}

/** @param {Grinder} g */
function sharpen(g) {
  return g.grind(1) + g.coarse();
  //       ^ d: coffee/grinder.cjs:7
  //                     ^ d: coffee/grinder.cjs:17
}

function spin() {
  const m = new Mill(2);
  return m.turn + m.stones + Mill.grind + m.fast + new FineGrinder(1).grind(2) + Grinder.burr;
  //       ^ d: coffee/mill.cjs:10
  //                ^ d: coffee/mill.cjs:6
  //                              ^ d: none
  //                                        ^ d: coffee/mill.cjs:18
  //                                                                  ^ d: coffee/grinder.cjs:7
  //                                                                                     ^ d: none
}

module.exports = { sharpen, spin };
