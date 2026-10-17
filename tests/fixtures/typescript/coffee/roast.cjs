"use strict";

exports.darken = function (beans) {
  return beans + 1;
};

exports.Roaster = class Roaster {
  constructor(level) {
    this.level = level;
  }
};

/**
 * @typedef {Object} Bean
 * @property {string} origin Where it grew.
 * @property {number} scoops How many.
 */

/** @param {Bean} bean */
//          ^ d: coffee/roast.cjs:14
function measure(bean) {
  return bean.scoops + exports.darken(1);
  //          ^ d: coffee/roast.cjs:16
  //                           ^ d: coffee/roast.cjs:3
}

module.exports.measure = measure;
