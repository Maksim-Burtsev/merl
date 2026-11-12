import { Ewer, Shop, Stall, TallEwer, Urn } from './urn';

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

export function poured(stall: Stall) {
  new Ewer().size(2).named('a').boil();
  //                            ^ d: urnchain/urn.ts:42
  new TallEwer().size(2).boil();
  //                     ^ d: urnchain/urn.ts:56
  stall.maybeCounter()?.urn()?.boil();
  //                           ^ d: urnchain/urn.ts:2
  stall.maybeCounter()!.urn().boil();
  //                          ^ d: urnchain/urn.ts:2
  return stall.maybe()?.size(2).boil();
  //                            ^ d: urnchain/urn.ts:42
}
