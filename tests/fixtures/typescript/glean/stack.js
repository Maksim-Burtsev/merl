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
