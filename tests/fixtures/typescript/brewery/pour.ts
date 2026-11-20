import { Vat } from "./vat";
import { Decanter } from "./decanter";
import { Growler } from "./growler";

export function cast(v: Vat) {
  (<Decanter>v).tap();
//              ^ d: brewery/decanter.ts:4
  const j = <Decanter>v;
  j.tap();
//  ^ d: brewery/decanter.ts:4
}

export function split(x: Decanter | Growler) {
  if (x instanceof Decanter) {
    x.tap();
//    ^ d: brewery/decanter.ts:4
  } else {
    x.tap();
//    ^ d: picker brewery/decanter.ts:4, brewery/growler.ts:4, brewery/vat.ts:2; want brewery/growler.ts:4 (#1494)
  }
  x.tap();
//  ^ d: !jump
}
