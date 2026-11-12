export class Crate {
  heft = 0;
  build() {
    const box = {
      heft: 3,
      grow() {
        return this.heft;
//                  ^ d: literal_this.ts:5
      },
      shrink: function () {
        return this.heft;
//                  ^ d: literal_this.ts:5
      },
      outer: this.heft,
//                ^ d: picker literal_this.ts:2, …
    };
    return box;
  }
}

export const stash = {
  heft: 1,
  read() {
    if (this.heft > 0) {
//           ^ d: literal_this.ts:22
      return this.heft;
//                ^ d: literal_this.ts:22
    }
    return 0;
  },
};

export class Other {
  heft = 4;
}

export const tray = {
  heft: 2,
  lift() {
    return [1].map(function (x) { return this.heft + x; });
//                                            ^ d: !jump
  },
  pack() {
    return { heft: 9, get() { return this.heft; } };
//                                        ^ d: !jump
  },
  cart() {
    return new (class { heft = 7; get() { return this.heft; } })();
//                                                    ^ d: !jump
  },
  shrink: function (this: Other) {
    return this.heft;
//              ^ d: literal_this.ts:34
  },
  grow(this: Other) {
    return this.heft;
//              ^ d: !jump
  },
};
