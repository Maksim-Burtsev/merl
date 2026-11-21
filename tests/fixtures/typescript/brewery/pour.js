import { Decanter } from "./decanter";

// JavaScript has no angle-bracket cast: `(<Decanter>v)` types nothing (#1494 review).
export function castInJs(v) {
  (<Decanter>v).tap();
  //            ^ d: picker brewery/decanter.ts:4, brewery/growler.ts:4, brewery/vat.ts:2
}
