import "./hoard";
import { Pile } from "./stack";

export function g(xs: number[], ys: Array<string>, p: Pile) {
  xs.penult();
//   ^ d: rinse/hoard.ts:5
  ys.penult();
//   ^ d: rinse/hoard.ts:5
  p.penult();
//  ^ d: rinse/stack.ts:2
}
