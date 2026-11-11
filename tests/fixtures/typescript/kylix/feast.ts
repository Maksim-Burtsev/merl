import { Kylix } from "./kylix";
import { Skyphos, Rhyton, Kantharos } from "./vessels";

export function feast(): void {
  const c = new Kylix();
  c.libate();
  //^ d: kylix/kylix.ts:1
  const s = new Skyphos();
  s.libate();
  //^ d: kylix/vessels.ts:1
  const r = new Rhyton();
  r.libate();
  //^ d: kylix/vessels.ts:2
  const k = new Kantharos();
  k.libate();
  //^ d: kylix/vessels.ts:4
}
