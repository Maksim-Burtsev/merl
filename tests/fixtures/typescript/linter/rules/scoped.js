class Decoy {
  variables = 1;
  references = 2;
  tokens = 3;
}

/** @type {import('../types').Rule.RuleModule} */
module.exports = {
  create(context) {
    const sourceCode = context.sourceCode;
    const scope = sourceCode.getScope(context);
    scope.variables;
    //    ^ d: picker linter/rules/scoped.js:2, linter/types/index.d.ts:47; want linter/types/index.d.ts:47 (#1337)
    sourceCode.getScope(context).variables;
    //                           ^ d: picker linter/rules/scoped.js:2, linter/types/index.d.ts:47; want linter/types/index.d.ts:47 (#1337)
    sourceCode.getOuter(context).references;
    //                           ^ d: picker linter/rules/scoped.js:3, linter/types/index.d.ts:51; want linter/types/index.d.ts:51 (#1337)
    sourceCode.ast.tokens;
    //             ^ d: linter/types/index.d.ts:40
    return new Decoy();
  },
};
