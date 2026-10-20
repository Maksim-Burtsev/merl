"use strict";

// A visitor whose name is a string or a computed key opens a method like any other (#339).
module.exports = {
  create(context) {
    return {
      "NewExpression:exit"(probe) {
        return probe.callee;
        //     ^ d: rules/q.js:7
        // status: local
        //           ^ d: node_modules/@types/estree/index.d.ts:6
      },
      'Program:exit'(probe) {
        return probe;
        //     ^ d: rules/q.js:13
      },
      [Symbol.iterator](probe) {
        return probe;
        //     ^ d: rules/q.js:17
      },
      CallExpression(probe) {
        return probe.callee;
        //     ^ d: rules/q.js:21
        //           ^ d: picker @types/estree/index.d.ts:6
      },
    };
  },
};
