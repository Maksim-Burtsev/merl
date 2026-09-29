const { weigh } = require("./warehouse");
const warehouse = require("./warehouse");

class Courier {
  constructor(name) {
    this.name = name;
  }
}

function load(grams) {
  return weigh(grams) + warehouse.weigh(grams);
  //     ^ d: shop/legacy.js:1; want shop/warehouse.ts:11 (#328)
  //                              ^ d: none; want shop/warehouse.ts:11 (#328)
}

module.exports = { Courier, load };
