/** @type {import('../types').Rule.RuleModule} */
module.exports = {
  create(context) {
    const sourceCode = context.sourceCode;
    //                         ^ d: node_modules/@lint/core/index.d.ts:2
    context.report({});
    //      ^ d: node_modules/@lint/core/index.d.ts:4
    return sourceCode.getFirstToken(context);
    //                ^ d: linter/types/index.d.ts:6
  },
};
