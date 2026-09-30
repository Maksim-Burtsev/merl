"use strict";

// CommonJS (#328): a `require` binds like an import, and a `const` continued over lines binds
// every name it declares.
const { helper } = require("./utils/m");
const utils = require("./utils/m");
const Segment = require("./utils/seg");
const fs = require("node:fs"),
  more = require("./utils/m"),
  LIMIT = 10;
const debug = require("debug")("app");
const { helper: assist } = require("./utils/m");
const {
  Other,
} = require("./utils/m");
const parts = require("./utils/parts");

function drive(node) {
  const a = helper(node);
  //        ^ d: cjs/utils/m.js:3
  //        status: via import cjs/utils/m.js
  const b = utils.helper(node);
  //              ^ d: cjs/utils/m.js:3
  const c = Segment.make();
  //        ^ d: cjs/utils/seg.js:3
  //                ^ d: cjs/utils/seg.js:4
  const d = more.helper(LIMIT);
  //        ^ d: cjs/utils/m.js:11
  //             ^ d: cjs/utils/m.js:3
  //                    ^ d: cjs/a.js:10
  //                    status: local
  const e = debug(node) + assist(node) + Other.make();
  //        ^ d: cjs/a.js:11
  //                      ^ d: cjs/utils/m.js:3
  //                                     ^ d: cjs/utils/m.js:7
  //                                           ^ d: cjs/utils/m.js:8
  const f = parts.part() + parts.whole();
  //        ^ d: cjs/utils/parts.js:1
  //        status: module cjs/utils/parts.js
  //              ^ d: cjs/utils/parts.js:3
  return [a, b, c, d, e, f, fs, utils];
  //                            ^ d: cjs/utils/m.js:11
}

function wrapped(node) {
  const first = node.first,
    /*
     * A comment between declarators, with commas, and a value on the line below its name.
     */
    second =
      node.second,
    third = first + second;
  return second + third;
  //     ^ d: cjs/a.js:50
  //              ^ d: cjs/a.js:52
}

module.exports = { drive, wrapped };
