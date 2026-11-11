import { Barrel, Crate, Gantry } from './gantry';

export class Shelf extends Gantry<Crate> {
  tidy(): void {
    this.first().fill();
    //           ^ d: gantry/gantry.ts:14
  }
}

export class Tall extends Shelf {}

export class Bracket<U> extends Gantry<U> {
  tidy(): void {
    this.first().fill();
    //           ^ d: picker gantry/gantry.ts:14, gantry/gantry.ts:18
  }
}

export class Loose extends Gantry {
  tidy(): void {
    this.first().fill();
    //           ^ d: picker gantry/gantry.ts:14, gantry/gantry.ts:18
  }
}

export function use(s: Shelf, t: Tall): void {
  s.first().fill();
  //        ^ d: gantry/gantry.ts:14
  t.first().fill();
  //        ^ d: gantry/gantry.ts:14
  s.load.fill();
  //     ^ d: gantry/gantry.ts:14
  s.grab(new Barrel()).fill();
  //                   ^ d: picker gantry/gantry.ts:14, gantry/gantry.ts:18
}

declare function shelf(): Shelf;

export function made(): void {
  shelf().first().fill();
  //              ^ d: gantry/gantry.ts:14
}
