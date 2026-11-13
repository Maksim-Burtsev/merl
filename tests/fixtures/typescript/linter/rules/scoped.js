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
    //    ^ d: linter/types/index.d.ts:48
    sourceCode.getScope(context).variables;
    //                           ^ d: linter/types/index.d.ts:48
    sourceCode.getOuter(context).references;
    //                           ^ d: linter/types/index.d.ts:52
    sourceCode.getLost(context).variables;
    //                          ^ d: picker linter/rules/scoped.js:2, linter/types/index.d.ts:48
    sourceCode.ast.tokens;
    //             ^ d: linter/types/index.d.ts:41
    return new Decoy();
  },
};
