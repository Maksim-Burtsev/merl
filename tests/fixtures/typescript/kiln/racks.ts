import { Kiln } from './kiln';

class Pot {
  boil(): number {
    return 3;
  }
}

export function racked(
  byName: Record<string, Kiln>,
  loose: Record<string, any>,
  grid: Record<string, Kiln[]>,
  pots: Pot[],
) {
  byName['a'].boil();
  //          ^ d: kiln/kiln.ts:2
  loose['a'].boil();
  //         ^ d: !jump
  grid['a'].boil();
  //        ^ d: !jump
  pots[0].boil();
  //      ^ d: kiln/racks.ts:4
  for (const k in byName) {
    // @ts-expect-error `in` hands out keys
    k.boil();
    //^ d: !jump
  }
}
