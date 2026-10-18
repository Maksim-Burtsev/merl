"use strict";

const Bin = require("./bin.cjs");

module.exports = class Hopper extends Bin {
  constructor(beans) {
    super();
    this.beans = beans;
    this.index = 0;
  }

  next() {
    this.current = this.beans[this.index];
    //   ^ d: coffee/bin.cjs:5
    //                  ^ d: coffee/hopper.cjs:8
    //                             ^ d: coffee/hopper.cjs:9
    return this.current + this.level;
    //                         ^ d: none
  }
};
