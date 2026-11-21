import { Carafe as Flagon } from "./carafe";

export function aliased(): void {
  Flagon.decant();
//       ^ d: statics/carafe.ts:2
  Flagon.tip().swirl();
//             ^ d: statics/carafe.ts:8
}
