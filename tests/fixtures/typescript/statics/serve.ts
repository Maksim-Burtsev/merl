import { Carafe } from "./carafe";
import { Ewer } from "./ewer";

export function serve(c: Carafe): void {
  Carafe.decant();
//       ^ d: statics/carafe.ts:2
  Ewer.decant();
//     ^ d: statics/ewer.ts:2
  c.decant(2);
//  ^ d: statics/carafe.ts:5
  Carafe.decant().swirl();
//                ^ d: statics/carafe.ts:8
  Carafe.tip().swirl();
//             ^ d: statics/carafe.ts:8
}

export function named(Carafe: Carafe): void {
  Carafe.decant(2);
//       ^ d: !jump
  Carafe.tip(2).swirl();
//              ^ d: !jump
}

export function poured(): void {
  Ewer.decant().swirl();
//              ^ d: statics/ewer.ts:5
}

export class Tray {
  Carafe: Carafe = new Carafe();
  pour(): void {
    this.Carafe.decant(2);
//              ^ d: statics/carafe.ts:5
  }
}

export function held(): void {
  const K = Carafe;
  K.decant();
//  ^ d: !jump
}

export function classed(k: typeof Carafe): void {
  k.decant();
//  ^ d: !jump
}

export function kept(c: Carafe): void {
  const d = c;
  d.decant(2);
//  ^ d: statics/carafe.ts:5
  c.decant();
//  ^ d: statics/carafe.ts:5
}

function maker(): typeof Carafe {
  return Carafe;
}

export function made(): void {
  const K: typeof Carafe = Carafe;
  K.decant();
//  ^ d: !jump
  maker().decant();
//        ^ d: !jump
  new Carafe().decant(2);
//             ^ d: statics/carafe.ts:5
  Carafe.decant().decant(2);
//                ^ d: statics/carafe.ts:5
}
