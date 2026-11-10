import { Kiln, TallKiln, Yard } from './kiln';

class Pot {
  boil(): number {
    return 3;
  }
}

export async function fired(yard: Yard, kiln: Kiln, tall: TallKiln) {
  (await yard.find()).boil();
  //                  ^ d: kiln/kiln.ts:2
  (await yard.fetch()).named('a').boil();
  //                   ^ d: kiln/kiln.ts:10
  //                              ^ d: kiln/kiln.ts:2
  const k = await yard.find();
  k.boil();
  //^ d: kiln/kiln.ts:2
  yard.find().then((f) => f.boil());
  //          ^ d: !jump
  const p = yard.fetch();
  p.boil();
  //^ d: !jump
  (await yard.loose()).boil();
  //                   ^ d: !jump
  (await yard.vague()).boil();
  //                   ^ d: !jump
  kiln.size(2).named('a').boil();
  //                      ^ d: kiln/kiln.ts:2
  tall.size(2).boil();
  //           ^ d: kiln/kiln.ts:16
  return new Pot().boil();
  //               ^ d: kiln/main.ts:4
}
