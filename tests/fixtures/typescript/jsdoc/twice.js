"use strict";

/**
 * @typedef {Object} Twin
 * @property {number} twinSlot The first.
 */

/**
 * @typedef {Object} Twin
 * @property {number} twinSlot The second.
 */

/** @type {Twin} */
const twin = load();
module.exports = twin.twinSlot;
//                    ^ d: picker jsdoc/twice.js:5, jsdoc/twice.js:10
