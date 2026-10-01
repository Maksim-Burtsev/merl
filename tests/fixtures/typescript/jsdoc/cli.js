"use strict";

/** @typedef {import("./options").ParsedFlags} ParsedFlags */
const Engine = require("./engine");

function run(args) {
	/** @type {ParsedFlags} */
	let flags;
	flags = parse(args);
	return flags.warnLimit;
	//           ^ d: jsdoc/options.js:5
	//             status: warnLimit → ParsedFlags.warnLimit (via flags: ParsedFlags)
}

/**
 * @param {ParsedFlags} flags The CLI flags.
 * @returns {string} The style.
 */
function styleOf(flags) {
	return flags.outStyle;
	//           ^ d: jsdoc/options.js:6
}

/**
 * @param {?ParsedFlags} [flags] May be missing.
 * @param {Engine} engine The engine.
 */
function marks(flags, engine) {
	Engine.stewardOf(engine).popMark();
	//                       ^ d: jsdoc/steward.js:4
	const steward = Engine.stewardOf(engine);
	steward.popMark();
	//      ^ d: jsdoc/steward.js:4
	return flags.warnLimit;
	//           ^ d: jsdoc/options.js:5
}

/**
 * @param {ParsedFlags|Engine} both A union types nothing.
 * @param {Map<string, ParsedFlags>} byName Nor does a generic.
 * @param {string} byName.outStyle Nor a part of a parameter.
 */
function refused(both, byName) {
	/** @type {ParsedFlags} */

	let loose = parse();
	return both.warnLimit + byName.outStyle + loose.outStyle;
	//          ^ d: picker jsdoc/options.js:5, jsdoc/types/index.d.ts:3
	//                             ^ d: picker jsdoc/options.js:6, jsdoc/types/index.d.ts:7
	//                                              ^ d: picker jsdoc/options.js:6, jsdoc/types/index.d.ts:7
}

module.exports = { run, styleOf, marks, refused };

/** @typedef {import("./options").SpareFlags} SpareFlags */

/**
 * @param {SpareFlags} spare Lower case `object`, `@prop` and a default.
 * @param {ParsedFlags} opts.flags A part of a parameter types nothing.
 */
function slots(spare, opts) {
	/** @type {any} */
	const loose = load();
	/** @type {import("./options").ParsedFlags} */
	const inline = load();
	return spare.shortSlot + spare.longSlot + opts.warnLimit + loose.outStyle + inline.outStyle;
	//           ^ d: jsdoc/options.js:14
	//                             ^ d: jsdoc/options.js:15
	//                                             ^ d: picker jsdoc/options.js:5, jsdoc/types/index.d.ts:3
	//                                                               ^ d: picker jsdoc/options.js:6, jsdoc/types/index.d.ts:7
	//                                                                                 ^ d: picker jsdoc/options.js:6, jsdoc/types/index.d.ts:7
}

module.exports.slots = slots;
