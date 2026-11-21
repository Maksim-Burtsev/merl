import { Vat } from "./vat";
import { Decanter } from "./decanter";

// An `.mts` file reads the angle-bracket cast as a `.ts` one does (#1494 review).
export function castInMts(v: Vat) {
  (<Decanter>v).tap();
  //            ^ d: brewery/decanter.ts:4
}
