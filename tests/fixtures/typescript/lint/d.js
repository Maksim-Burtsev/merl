"use strict";

// An optional signature wrapped over lines, a field under a header wrapped at a type argument,
// and method headers wrapped after `(` or opening with a pattern are declarations (#343).
function walk(listener, sourceCode, painter) {
  listener.onCodePathEnd(1, 2);
  //       ^ d: lint/types/index.d.ts:2
  painter.render(painter.paint({ a: 1, b: 2 }));
  //      ^ d: lint/shapes.ts:2
  //                     ^ d: lint/shapes.ts:8
  return sourceCode.getTokenAfter(1);
  //                ^ d: lint/types/index.d.ts:11
}

module.exports = { walk };

// A method whose parameter's type literal holds `;` is a declaration (#528).
function sketch(easel) {
  return easel.stroke({ a: 1, b: 2 });
  //           ^ d: lint/shapes.ts:17
}
