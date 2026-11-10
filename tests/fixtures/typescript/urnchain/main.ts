import { Shop, Urn } from './urn';

class Pot {
  boil(): number {
    return 2;
  }
}

export function run(shop: Shop) {
  Urn.builder().size(2).build().boil();
  //            ^ d: urnchain/urn.ts:12
  //                    ^ d: urnchain/urn.ts:16
  //                            ^ d: urnchain/urn.ts:2
  shop.counter().urn().boil();
  //             ^ d: urnchain/urn.ts:22
  //                   ^ d: urnchain/urn.ts:2
  shop.mystery().urn().boil();
  //             ^ d: !jump
  //                   ^ d: !jump
  Urn.builder()
    .size(2)
    //^ d: urnchain/urn.ts:12
    .build()
    //^ d: urnchain/urn.ts:16
    .boil();
    //^ d: urnchain/urn.ts:2
  return new Pot().boil();
}

export function loosely(shop: Shop) {
  return shop.counter().loose().boil();
  //                            ^ d: !jump
}
