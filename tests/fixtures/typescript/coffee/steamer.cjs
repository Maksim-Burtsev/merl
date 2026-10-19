"use strict";

function Steamer(bar) {
  this.bar = bar;
  const self = this;
  this.vent = function () {
    return self.bar;
    //          ^ d: none; want coffee/steamer.cjs:4 (#1149)
  };
}

Steamer.prototype.purge = function () {
  const that = this;
  return [1].map(function () {
    return that.bar;
    //          ^ d: none; want coffee/steamer.cjs:4 (#1149)
  });
};

module.exports = { Steamer, ...require("./roast.cjs") };
