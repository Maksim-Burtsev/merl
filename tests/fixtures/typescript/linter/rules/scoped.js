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
      //   ^ d: linter/types/index.d.ts:47
    }
    while ((next = queue.shift()) !== undefined) {
      next.childScopes;
      //   ^ d: linter/types/index.d.ts:47
    }
    const roots = [sourceCode.getScope(context)];
    for (const root of roots) {
      root.variables;
      //   ^ d: linter/types/index.d.ts:48
    }
    const decoy = new Decoy();
    const held = [decoy];
    held.forEach(one => one.childScopes);
    //                      ^ d: linter/rules/scoped.js:6
    let found;
    while ((found = held.pop())) {
      if (found) {
        found = scope;
      }
      found.childScopes;
      //    ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    const pair = [scope, new Decoy()];
    while ((next = pair.pop())) {
      next.childScopes;
      //   ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    const spread = [...scope.childScopes];
    while ((next = spread.pop())) {
      next.childScopes;
      //   ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    const decoys = [new Decoy()];
    while ((next = queue.pop())) {
      next = decoys.pop();
      next.childScopes;
      //   ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    const stack = new Decoy();
    while ((next = stack.pop())) {
      next.childScopes;
      //   ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    while ((next = decoys.pop())) {
      next.childScopes;
      //   ^ d: linter/rules/scoped.js:6
    }
    const walked = [sourceCode.getScope(context)];
    while ((next = walked.pop())) {
      walked.push(sourceCode.getScope(context));
      next.childScopes;
      //   ^ d: linter/types/index.d.ts:47
    }
    const listed = [scope];
    listed.push(new Decoy());
    for (const one of listed) {
      one.childScopes;
      //  ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    const piled = [scope];
    while ((next = piled.pop())) {
      piled.push(new Decoy());
      next.childScopes;
      //   ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    let swapped = [scope];
    swapped = [new Decoy()];
    for (const one of swapped) {
      one.childScopes;
      //  ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    const slots = [scope];
    slots[0] = new Decoy();
    slots[0].childScopes;
    //       ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    while ((next = walked.pop())) {
      [next] = [new Decoy()];
      next.childScopes;
      //   ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    if (scope) {
      var mixed = [scope];
    } else {
      var mixed = [new Decoy()];
    }
    for (const one of mixed) {
      one.childScopes;
      //  ^ d: picker linter/rules/scoped.js:6, linter/types/index.d.ts:47
    }
    return new Decoy();
  },
};
