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
//                ^ d: picker snifter/snifter.ts:4, statics/carafe.ts:8, statics/ewer.ts:5; want statics/carafe.ts:8 (#1489)
}

export function named(Carafe: Carafe): void {
  Carafe.decant(2);
//       ^ d: !jump
}

export class Tray {
  Carafe: Carafe = new Carafe();
  pour(): void {
    this.Carafe.decant(2);
//              ^ d: !jump
  }
}
