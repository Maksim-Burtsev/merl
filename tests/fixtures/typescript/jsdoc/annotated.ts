/**
 * @param {ParsedFlags} flags A TypeScript file writes annotations, not JSDoc.
 */
export function limit(flags) {
	return flags.warnLimit;
	//           ^ d: picker jsdoc/options.js:5, jsdoc/types/index.d.ts:3
}
