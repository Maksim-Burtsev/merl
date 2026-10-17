"use strict";

exports.timeout = (process.env.T || 5);
exports.pour = async (cup) => cup;
exports.stir = spoon => spoon;
module.exports.brew = function brew(cup) {
  return exports.stir(cup);
  //             ^ d: coffee/sieve.cjs:5
};
