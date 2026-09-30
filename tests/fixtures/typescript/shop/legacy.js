const { weigh } = require("./warehouse");
const warehouse = require("./warehouse");

class Courier {
  constructor(name) {
    this.name = name;
  }
}

function load(grams) {
  return weigh(grams) + warehouse.weigh(grams);
  //     ^ d: shop/warehouse.ts:11
  //                              ^ d: shop/warehouse.ts:11
}

module.exports = { Courier, load };

function flag(context, lastItem) {
  context.report({ node: lastItem });
  //               ^ d: none
  //               status: node: key
}
