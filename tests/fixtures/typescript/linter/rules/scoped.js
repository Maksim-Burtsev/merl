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
    //    ^ d: none
    sourceCode.getScope(context).variables;
    //                           ^ d: none
    sourceCode.getOuter(context).references;
    //                           ^ d: none
    sourceCode.ast.tokens;
    //             ^ d: none
    return new Decoy();
  },
};
