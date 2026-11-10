"use strict";

/**
 * @typedef {Object} VerifyOptions
 * @property {string} [filename] The file.
 * @property {boolean} [allowInlineConfig] Inline comments.
 * @property {string} [reportUnused] Severity.
 */

/**
 * @typedef {Object} InternalOptions
 * @property {string | null} warnInlineConfig Where it came from.
 * @property {string} reportUnused Severity, normalized.
 */

/**
 * @typedef {Object} FileInfo
 * @property {string} filename A namesake.
 * @property {string} warnInlineConfig A namesake.
 */

/**
 * @param {VerifyOptions} provided Options.
 * @returns {Required<VerifyOptions> & InternalOptions} Normalized options.
 */
function normalize(provided) {
	return Object.assign({}, provided);
}

/**
 * @param {VerifyOptions} provided Options.
 * @returns {VerifyOptions & Unknown} One arm nobody declares.
 */
function halfKnown(provided) {
	return Object.assign({}, provided);
}

function verify(provided) {
	const options = normalize(provided);
	const half = halfKnown(provided);
	return [
		options.filename,
		//      ^ d: jsdoc/verify.js:5
		options.warnInlineConfig,
		//      ^ d: jsdoc/verify.js:12
		options.reportUnused,
		//      ^ d: picker jsdoc/verify.js:7, jsdoc/verify.js:13
		half.filename,
		//   ^ d: picker jsdoc/verify.js:5, jsdoc/verify.js:18
	];
}

/** @type {FileInfo} */
const info = { filename: "", warnInlineConfig: "" };

module.exports = { verify, info };
