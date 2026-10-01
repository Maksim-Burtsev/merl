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
