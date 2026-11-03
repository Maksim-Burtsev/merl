"use strict";

/** @typedef {Object} Row */
/** @typedef {Row[] & { duration: bigint; laps?: Record<string, number>; }} Results */
/** @typedef {Row[] | { laps: number; }} Either */
/** @typedef {Array<{ laps: number; }>} Wrapped */
/** @typedef {{ laps: number; } & { laps: string; }} Both */

class Track {
	constructor() {
		this.laps = 0;
	}
}

/** @type {Results} */
const results = [];
results.laps = {};
//      ^ d: jsdoc/joined.js:4
results.duration = 0n;
//      ^ d: jsdoc/joined.js:4
results.push({});
//      ^ d: !jump

/** @type {Results} */
const marked = [];
const unmarked = [];
unmarked.laps = {};
//       ^ d: !jump

/** @type {Either} */
const either = [];
either.laps = 1;
//     ^ d: !jump

/** @type {Wrapped} */
const wrapped = [];
wrapped.laps = 1;
//      ^ d: !jump

/** @type {Both} */
const both = {};
both.laps = 1;
//   ^ d: !jump

module.exports = { marked, Track };

/** @typedef {Row[] & { laps: number; }} Lapped */
class Lapped {
	constructor() {
		this.laps = 0;
	}
}
/** @type {Lapped} */
const lapped = [];
lapped.laps = 1;
//     ^ d: jsdoc/joined.js:50
