import { Kiln } from './kiln';

class Shelf {
  private byName: Map<string, Kiln> = new Map();

  heat(): number {
    return this.byName.get('a')!.boil();
    //                           ^ d: kiln/kiln.ts:2
  }
}

export function mapped(
  byName: Map<string, Kiln>,
  seen: ReadonlyMap<string, Kiln>,
  grid: Map<string, Kiln[]>,
) {
  byName.get('a')!.boil();
  //               ^ d: kiln/kiln.ts:2
  byName.get('a')?.boil();
  //               ^ d: kiln/kiln.ts:2
  seen.get('a')!.boil();
  //             ^ d: kiln/kiln.ts:2
  grid.get('a')![0].boil();
  //                ^ d: !jump
}
