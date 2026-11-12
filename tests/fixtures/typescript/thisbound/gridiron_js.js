export class GridironJs {
  sear() {
    return 2;
  }

  viaFunction(items) {
    items.forEach(function () { this.sear(); });
    //                               ^ d: picker thisbound/gridiron.ts:4, thisbound/griddle.ts:2, thisbound/gridiron_js.js:2
    items.forEach(function () {
      this.sear();
    //     ^ d: picker thisbound/gridiron.ts:4, thisbound/griddle.ts:2, thisbound/gridiron_js.js:2
    });
    items.forEach(() => { this.sear(); });
    //                         ^ d: thisbound/gridiron_js.js:2
  }
}
