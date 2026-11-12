import { Carafe } from "./carafe";
import { Ewer } from "./ewer";

export function serve(c: Carafe): void {
  Carafe.decant();
//       ^ d: statics/carafe.ts:2
  Ewer.decant();
//     ^ d: statics/ewer.ts:2
  c.decant(2);
//  ^ d: picker statics/carafe.ts:2, statics/carafe.ts:5; want statics/carafe.ts:5 (#1489)
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
//              ^ d: !jump
  }
}
