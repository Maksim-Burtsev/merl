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
