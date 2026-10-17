"use strict";

const Grinder = require("./grinder.cjs");
const { Roaster } = require("./roast.cjs");

/** @typedef {string} Origin */

/** @param {Grinder} g */
//          ^ d: coffee/grinder.cjs:3
/** @param {Origin} o */
//          ^ d: coffee/brew.cjs:6
/** @param {Roaster} r */
//          ^ d: coffee/roast.cjs:7
/** @param {Kettle} k */
//          ^ d: picker coffee/kettle.mjs:1, tea/kettle.ts:1
/** @returns {Origin} */
//            ^ d: coffee/brew.cjs:6
function brew(g, o, r, k) {
  return o + g + r + k;
}

module.exports = brew;
