/**
 * @typedef {Object} Receipt
 * @property {string} total What the customer paid
 * @property {number} lines How many lines it has
 */

/** @type {Receipt} */
const receipt = JSON.parse(localStorage.getItem("receipt"));

export const paid = receipt.total;
