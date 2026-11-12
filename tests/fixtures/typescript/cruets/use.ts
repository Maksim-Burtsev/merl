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
  //^ d: cruets/cruet.ts:2
  if (c instanceof Phial && ok) {
    c.uncork();
    //^ d: cruets/cruet.ts:2
  }
  const v = c instanceof Vial ? c.stopper() : 0;
  //                              ^ d: cruets/cruet.ts:3
  void v;
}

export function later(c: Cruet): void {
  if (c instanceof Vial) {
    [1].forEach(() => {
      c.uncork();
      //^ d: cruets/cruet.ts:2
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
    //^ d: cruets/cruet.ts:2
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
  //       ^ d: cruets/cruet.ts:3
}

export class Rack {
  held: Cruet;

  constructor(held: Cruet) {
    this.held = held;
  }

  pour(): void {
    if (this.held instanceof Vial) {
      this.held.stopper();
      //        ^ d: cruets/vial.ts:5
    }
    this.held.stopper();
    //        ^ d: !jump
  }

  swap(p: Phial): void {
    if (this.held instanceof Vial) {
      this.held = p;
      this.held.uncork();
      //        ^ d: !jump
    }
  }
}

export function reassigned(c: Cruet, p: Phial, xs: Cruet[]): void {
  if (c instanceof Vial) {
    c = p;
    c.uncork();
    //^ d: cruets/cruet.ts:2
  }
  if (c instanceof Vial) {
    for (c of xs) {
      c.uncork();
      //^ d: cruets/cruet.ts:2
    }
  }
}

export function guardedThenReassigned(c: Cruet, p: Phial): void {
  if (!(c instanceof Vial)) return;
  c = p;
  c.uncork();
  //^ d: cruets/cruet.ts:2
}

export function outside(c: Cruet, ok: boolean): void {
  if (!(c instanceof Vial)) {
    console.log(ok);
  }
  c.uncork();
  //^ d: cruets/cruet.ts:2
  const v = c instanceof Vial ? 1 : c.stopper();
  //                                  ^ d: cruets/cruet.ts:3
  void v;
  if (c instanceof Map) {
    c.uncork();
    //^ d: cruets/cruet.ts:2
  }
  if (c instanceof Vial || ok) {
    c.uncork();
    //^ d: cruets/cruet.ts:2
  }
}

export function unrelated(p: Phial, both: Phial | Vial): void {
  if (p instanceof Vial) {
    p.uncork();
    //^ d: cruets/phial.ts:4
  }
  if (both instanceof Vial) {
    both.uncork();
    //   ^ d: cruets/vial.ts:4
  }
}
