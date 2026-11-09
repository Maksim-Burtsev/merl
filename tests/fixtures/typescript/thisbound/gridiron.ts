import { Griddle } from "./griddle";

export class Gridiron {
  sear(): number {
    return 2;
  }

  viaFunction(items: number[], g: Griddle) {
    items.forEach(function () { this.sear(); });
    //                               ^ d: picker thisbound/gridiron.ts:4, thisbound/griddle.ts:2, thisbound/gridiron_js.js:2
    items.forEach(function () { this.sear(); }, g);
    //                               ^ d: picker thisbound/gridiron.ts:4, thisbound/griddle.ts:2, thisbound/gridiron_js.js:2
    items.forEach(function (this: Griddle) { this.sear(); });
    //                                            ^ d: thisbound/griddle.ts:2
    const named = function () { return this.sear(); };
    //                                      ^ d: picker thisbound/gridiron.ts:4, thisbound/griddle.ts:2, thisbound/gridiron_js.js:2
    function inner() { return this.sear(); }
    //                             ^ d: picker thisbound/gridiron.ts:4, thisbound/griddle.ts:2, thisbound/gridiron_js.js:2
    function typed(this: Griddle) { return this.sear(); }
    //                                          ^ d: thisbound/griddle.ts:2
    const o = { run() { return this.sear(); } };
    //                              ^ d: picker thisbound/gridiron.ts:4, thisbound/griddle.ts:2, thisbound/gridiron_js.js:2
    items.forEach(function () {
      this.sear();
    //     ^ d: picker thisbound/gridiron.ts:4, thisbound/griddle.ts:2, thisbound/gridiron_js.js:2
    });
    function later() {
      return this.sear();
      //          ^ d: picker thisbound/gridiron.ts:4, thisbound/griddle.ts:2, thisbound/gridiron_js.js:2
    }
    return [named, inner, typed, o, later];
  }

  viaArrow(items: number[]) {
    items.forEach(() => { this.sear(); });
    //                         ^ d: thisbound/gridiron.ts:4
    items.forEach(() => {
      this.sear();
    //     ^ d: thisbound/gridiron.ts:4
    });
    console.log("seared", { ...this.sear(), at: 1 });
    //                              ^ d: thisbound/gridiron.ts:4
    items.map((n) => console.log({ at: n, heat: this.sear() }));
    //                                               ^ d: thisbound/gridiron.ts:4
    return this.sear();
    //          ^ d: thisbound/gridiron.ts:4
  }
}
