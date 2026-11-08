export class Rick {
  run(): void {
    this.bundleUp();
    //   ^ d: glean/stack.ts:10
    this.gatherIn();
    //   ^ d: glean/stack.ts:11
    this.lashTight();
    //   ^ d: glean/stack.ts:14
  }
  bundleUp() {} // a note
  gatherIn(): void { // a note
    this.bundleUp();
  }
  lashTight(): void { /* a note */ }
}
