class Decoy {
  variables = 1;
  references = 2;
  tokens = 3;
  range = 4;
  childScopes = 5;
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
    scope.variables.forEach(v => v.references);
    //                             ^ d: linter/types/index.d.ts:52
    sourceCode.ast.tokens.filter(t => t.range);
    //                                  ^ d: linter/types/index.d.ts:37
    scope.variables.forEach(v => {
      v.references;
      //^ d: linter/types/index.d.ts:52
    });
    scope.variables.forEach(variable => {
      if (variable) {
        variable.references;
        //       ^ d: linter/types/index.d.ts:52
      }
    });
    sourceCode.ast.tokens.filter(
      token =>
        token.kind ||
        (token &&
          token.range),
      //        ^ d: linter/types/index.d.ts:37
    );
    sourceCode.ast.tokens.reduce(
      token =>
        token.range,
      //      ^ d: picker linter/rules/scoped.js:5, linter/types/index.d.ts:37
    );
    const queue = [sourceCode.getScope(context)];
    let next;
    while ((next = queue.pop())) {
      next.childScopes;
      //   ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47; want linter/types/index.d.ts:47 (#1337)
    }
    return new Decoy();
  },
};
