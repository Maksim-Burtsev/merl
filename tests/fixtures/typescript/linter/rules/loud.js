module.exports = {
  create(context) {
    context.report({});
    //      ^ d: picker linter/token_store.js:6, @lint/core/index.d.ts:4
  },
};

/** @type {import('../types').Rule.RuleModule} */
const other = {
  meta: {},
  create(context, extra) {
    extra.report({});
    //    ^ d: picker linter/token_store.js:6, @lint/core/index.d.ts:4
  },
};

/** @type {import('../types').Rule.RuleModule | import('../types').Rule.RuleContext} */
const either = {
  create(context) {
    context.report({});
    //      ^ d: picker linter/token_store.js:6, @lint/core/index.d.ts:4
  },
};

/** @type {import('../types').Rule.RuleModule} */
const chained = {
  create(context) {
    const sourceCode = context.sourceCode
      .getFirstToken(context);
    return sourceCode.getFirstToken(context);
    //                ^ d: picker linter/token_store.js:2, linter/types/index.d.ts:6
  },
};
