"use strict";

function h(input) {
  let probe = input;
  const callee = probe.callee;
  return callee;
}

module.exports = { h };
