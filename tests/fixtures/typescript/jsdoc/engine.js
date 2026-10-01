"use strict";

const Steward = require("./steward");

class Engine {
	/**
	 * The steward of an engine.
	 * @param {Engine} engine The engine.
	 * @returns {Steward} Its steward.
	 */
	static stewardOf(engine) {
		return engine.steward;
	}
}

module.exports = Engine;

/** @type {Steward[]} */
const crew = [];

function sweep() {
	for (const member of crew) {
		member.popMark();
		//     ^ d: jsdoc/steward.js:4
	}
}

/** @returns {Object} An untyped promise leaves the body to say. */
function fresh() {
	return new Steward();
}

/** @return {Steward} The short tag. */
function keeper() {
	return load();
}

/** @type {Array<Steward>} */
const pool = [];

/**
 * @param {Object} [held] A type nothing declares keeps the default's.
 * @param {Steward=} maybe An optional one.
 * @param {Steward|null} nullable A nullable one.
 * @param {Steward} [spare=keeper()] One with a default.
 */
function forms(held = new Steward(), maybe, nullable, spare) {
	/** @type {Steward} */ let solo = load();
	const made = new Steward();
	fresh().popMark();
	//      ^ d: jsdoc/steward.js:4
	keeper().popMark();
	//       ^ d: jsdoc/steward.js:4
	held.popMark();
	//   ^ d: jsdoc/steward.js:4
	maybe.popMark();
	//    ^ d: jsdoc/steward.js:4
	nullable.popMark();
	//       ^ d: jsdoc/steward.js:4
	spare.popMark();
	//    ^ d: jsdoc/steward.js:4
	solo.popMark();
	//   ^ d: jsdoc/steward.js:4
	made.popMark();
	//   ^ d: jsdoc/steward.js:4
	for (const one of pool) {
		one.popMark();
		//  ^ d: jsdoc/steward.js:4
	}
}

class Deputy extends Steward {
	relieve() {
		return this.popMark();
		//          ^ d: jsdoc/steward.js:4
	}
}
