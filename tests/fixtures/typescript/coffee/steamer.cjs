"use strict";

function Steamer(bar) {
  this.bar = bar;
  const self = this;
  this.vent = function () {
    return self.bar;
    //          ^ d: coffee/steamer.cjs:4
  };
}

Steamer.prototype.purge = function () {
  const that = this;
  return [1].map(function () {
    return that.bar;
    //          ^ d: coffee/steamer.cjs:4
  });
};

module.exports = { Steamer, ...require("./roast.cjs") };
