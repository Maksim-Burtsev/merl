"use strict";

const { pour, stir, brew } = require("./sieve.cjs");
const sieve = require("./sieve.cjs");

module.exports = pour(stir(brew(sieve.pour)));
//               ^ d: coffee/sieve.cjs:4
//                    ^ d: coffee/sieve.cjs:5
//                         ^ d: coffee/sieve.cjs:6
//                                ^ d: coffee/sieve.cjs:1
//                                      ^ d: coffee/sieve.cjs:4
