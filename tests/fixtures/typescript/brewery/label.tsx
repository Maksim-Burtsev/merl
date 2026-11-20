import { Vat } from "./vat";
import { Decanter } from "./decanter";

export function Label(v: Vat) {
  const x = <Decanter>v
  </Decanter>;
  x.tap();
  //^ d: picker brewery/decanter.ts:4, brewery/growler.ts:4, brewery/vat.ts:2
  return x;
}
