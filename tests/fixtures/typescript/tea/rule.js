module.exports = {
  create(context) {
    return {
      ObjectExpression(node) {
        return node.properties;
        //          ^ d: node_modules/@types/estree/index.d.ts:14
      },
      "NewExpression:exit"(node) {
        return node.callee;
        //          ^ d: node_modules/@types/estree/index.d.ts:6
      },
      Property(node) {
        return node.hinge;
        //          ^ d: picker parcel-kit/index.d.ts:4
      },
    };
  },
};

export const plugin = {
  visitor: {
    ObjectExpression(path) {
      return path.properties;
      //          ^ d: picker @types/estree/index.d.ts:14, @types/estree/index.d.ts:39
    },
  },
};

exports.rule = {
  create(context) {
    return {
      ObjectExpression(node) {
        [context].forEach(function (node) {
          return node.properties;
          //          ^ d: picker @types/estree/index.d.ts:14, @types/estree/index.d.ts:39
        });
        node = context.other;
        return node.properties;
        //          ^ d: node_modules/@types/estree/index.d.ts:14
      },
    };
  },
};

exports.calls = {
  create(context) {
    return {
      CallExpression(node) {
        node.optional;
        //   ^ d: picker @types/estree/index.d.ts:27
        node.type;
        //   ^ d: !jump
        return node.callee;
        //          ^ d: node_modules/@types/estree/index.d.ts:6
      },
    };
  },
};

exports.parents = {
  create(context) {
    return {
      ObjectPattern(node) {
        return node.parent;
        //          ^ d: node_modules/eslint/lib/types/index.d.ts:5
      },
      "Property:exit"(node) {
        [context].forEach(function (node) {
          return node.parent;
          //          ^ d: chains.ts:13; want none (#1337)
        });
        return node.parent;
        //          ^ d: node_modules/eslint/lib/types/index.d.ts:5
      },
      Program(node) {
        return node.parent;
        //          ^ d: chains.ts:13; want none (#1337)
      },
    };
  },
};
