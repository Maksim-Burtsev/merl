"use strict";

const { Steamer, darken } = require("./steamer.cjs");

function froth() {
  const steamer = new Steamer(2);
  //                  ^ d: coffee/steamer.cjs:3
  return steamer.vent() + steamer.purge().length + darken(1);
  //             ^ d: coffee/steamer.cjs:6
  //                              ^ d: coffee/steamer.cjs:12
  //                                               ^ d: none; want coffee/roast.cjs:3 (#1150)
}

module.exports = froth;
