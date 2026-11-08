export class Rack {
  run() {
    this.bundleUp();
    //   ^ d: glean/stack.js:10
    this.gatherIn();
    //   ^ d: glean/stack.js:11
    this.lashTight();
    //   ^ d: glean/stack.js:14
  }
  bundleUp() {} // a note
  gatherIn() { // a note
    this.bundleUp();
  }
  lashTight() { /* a note */ }
}

export class Twine {
  knot() {}
}

export function tie(knot) {
  knot({ // a call, no declaration
    n: 1,
  });
}

export function pull() {
  return this.knot();
  //          ^ d: glean/stack.js:18
}
