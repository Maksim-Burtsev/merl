/**
 * @param {ParsedFlags} flags A TypeScript file writes annotations, not JSDoc.
 */
export function limit(flags) {
	return flags.warnLimit;
	//           ^ d: picker jsdoc/options.js:5, jsdoc/types/index.d.ts:3
}

class Keeper {
	popMark() {
		return 2;
	}
}

/**
 * @param {Keeper} keeper A type the file resolves, read all the same as no annotation.
 */
export function poke(keeper) {
	return keeper.popMark();
	//            ^ d: picker jsdoc/steward.js:4, jsdoc/types/index.d.ts:11, jsdoc/annotated.ts:10
}

/**
 * @typedef {Object} TsOnly
 * @property {string} tsSlot A typedef of a `.ts` file declares no field.
 //                   ^ d: none
 */
export class Holder {
	tsSlot = "";
}
