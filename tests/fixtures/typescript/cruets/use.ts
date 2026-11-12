import { Cruet } from "./cruet";
import { Phial } from "./phial";
import { Flute, Vial } from "./vial";

export function pour(c: Cruet, p: Phial, u: unknown): void {
  if (c instanceof Vial) {
    c.uncork();
    //^ d: cruets/vial.ts:4
  }
  if (c instanceof Phial) {
    c.stopper();
    //^ d: cruets/phial.ts:5
  } else if (c instanceof Vial) {
    c.stopper();
    //^ d: cruets/vial.ts:5
  } else {
    c.stopper();
    //^ d: cruets/cruet.ts:3
  }
  if (u instanceof Phial) {
    u.uncork();
    //^ d: cruets/phial.ts:4
  }
  c.uncork();
  //^ d: cruets/cruet.ts:2
  p.uncork();
  //^ d: cruets/phial.ts:4
}

export function guard(c: Cruet): number {
  if (!(c instanceof Vial)) return 0;
  return c.stopper();
  //       ^ d: cruets/vial.ts:5
}

export function guarded(c: Cruet): number {
  if (!(c instanceof Phial)) {
    return 0;
  }
  return c.stopper();
  //       ^ d: cruets/phial.ts:5
}

export function loose(c: Cruet, ok: boolean): void {
  if (!(c instanceof Vial)) {
    console.log(ok);
    return;
  }
  c.uncork();
  //^ d: !jump
  if (c instanceof Phial && ok) {
    c.uncork();
    //^ d: !jump
  }
  const v = c instanceof Vial ? c.stopper() : 0;
  //                              ^ d: !jump
  void v;
}

export function later(c: Cruet): void {
  if (c instanceof Vial) {
    [1].forEach(() => {
      c.uncork();
      //^ d: !jump
    });
    [2].forEach((c: Phial) => {
      c.uncork();
      //^ d: cruets/phial.ts:4
    });
  }
}

export function own(f: Flute): void {
  if (f instanceof Vial) {
    f.uncork();
    //^ d: cruets/vial.ts:11
  }
}

export function wrapped(c: Cruet): void {
  if (
    c instanceof Vial
  ) {
    c.stopper();
    //^ d: cruets/vial.ts:5
  }
  if (c instanceof Phial &&
      c.stopper() > 0) {
    c.uncork();
    //^ d: !jump
  }
}

export function otherwise(c: Cruet): number {
  if (c instanceof Vial) {
    c.uncork();
  } else {
    return 0;
  }
  return c.stopper();
  //       ^ d: cruets/vial.ts:5
}

export function noisy(c: Cruet): number {
  if (c instanceof Phial) {
    c.uncork();
  } else {
    console.log(c);
    return 0;
  }
  return c.stopper();
  //       ^ d: !jump
}
