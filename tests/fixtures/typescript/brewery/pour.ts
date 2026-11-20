import { Vat } from "./vat";
import { Decanter } from "./decanter";
import { Growler } from "./growler";

export function cast(v: Vat) {
  (<Decanter>v).tap();
  //            ^ d: brewery/decanter.ts:4
  const j = <Decanter>v;
  j.tap();
  //^ d: brewery/decanter.ts:4
}

export function split(x: Decanter | Growler) {
  if (x instanceof Decanter) {
    x.tap();
    //^ d: brewery/decanter.ts:4
  } else {
    x.tap();
    //^ d: brewery/growler.ts:4
  }
  x.tap();
  //^ d: !jump
}

export function chained(x: Decanter | Growler) {
  if (x instanceof Decanter) {
    x.tap();
  } else if (x instanceof Growler) {
    x.tap();
    //^ d: brewery/growler.ts:4
  } else {
    x.tap();
    //^ d: !jump
  }
}

export function three(x: Decanter | Growler | Vat) {
  if (x instanceof Decanter) {
    x.tap();
  } else {
    x.tap();
    //^ d: !jump
  }
}

export function outside(x: Vat | Growler) {
  if (x instanceof Decanter) {
    x.tap();
  } else {
    x.tap();
    //^ d: !jump
  }
}
