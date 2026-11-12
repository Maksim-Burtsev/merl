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
