"use strict";

const fs = require("fs");
const { join } = require("path");
import defaultExport, { a as b } from "./lib.js";

export const API_URL = "https://example.com";
const _HIDDEN = 1;
var legacy = null;
let nothing = undefined;

class Counter extends EventTarget {
  static #instances = 0;
  #count = 0;
  value = 0;

  constructor(start = 0) {
    super();
    this.#count = start;
    Counter.#instances++;
  }

  get count() {
    return this.#count;
  }

  increment(step = 1) {
    this.#count += step;
    this.dispatchEvent(new Event("change"));
    return this;
  }

  static create() {
    return new Counter();
  }
}

function* range(start, end) {
  for (let i = start; i < end; i++) {
    yield i;
  }
}

async function load(file) {
  try {
    const text = await fs.promises.readFile(join(__dirname, file), "utf8");
    return JSON.parse(text);
  } catch (err) {
    console.warn(`cannot load ${file}:`, err.message);
    return {};
  } finally {
    nothing = typeof legacy;
  }
}

const square = (x) => x * x;
const obj = {
  method() {
    return arguments.length;
  },
  [Symbol.toPrimitive]: () => 1,
  "quoted-key": true,
  nested: { deep: [1, 2.5, 0b101, 0o17, 1e-3] },
};

export default {
  Counter,
  range,
  load,
  square,
  obj,
  ok: obj?.nested?.deep ?? [],
  matches: /^[a-z]+$/i.test("abc"),
  same: 1 === 1 && 2 !== 3 || !false,
};

module.exports = { Counter, b, defaultExport, _HIDDEN, API_URL };
