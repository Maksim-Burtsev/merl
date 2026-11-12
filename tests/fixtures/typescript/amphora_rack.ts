import { Amphora } from "./amphora";

export class AmphoraRack<Amphora> {
//                       ^ d: amphora_rack.ts:3
  held: Amphora;
  //    ^ d: amphora_rack.ts:3
  take(x: Amphora): Amphora {
  //      ^ d: amphora_rack.ts:3
  //                ^ d: amphora_rack.ts:3
    return x;
  }
}

export function amphoraOne<Amphora>(x: Amphora): Amphora {
//                         ^ d: amphora_rack.ts:14
//                                     ^ d: amphora_rack.ts:14
//                                               ^ d: amphora_rack.ts:14
  return x;
}

export interface AmphoraHolder<Amphora> {
//                             ^ d: amphora_rack.ts:21
  get(): Amphora;
  //     ^ d: amphora_rack.ts:21
}

export type AmphoraPair<Amphora> = [Amphora, Amphora];
//                      ^ d: amphora_rack.ts:27
//                                  ^ d: amphora_rack.ts:27
//                                           ^ d: amphora_rack.ts:27

export function amphoraPlain(x: Amphora): Amphora {
//                              ^ d: amphora.ts:1
//                                        ^ d: amphora.ts:1
  return x;
}

export class AmphoraBound<T extends Amphora> {
//                        ^ d: amphora_rack.ts:38
//                                  ^ d: amphora.ts:1
  t: T;
  // ^ d: amphora_rack.ts:38
}

export class AmphoraOuter<Amphora> {
//                        ^ d: amphora_rack.ts:45
  inner<Amphora>(x: Amphora): Amphora {
  //    ^ d: amphora_rack.ts:47
  //                ^ d: amphora_rack.ts:47
  //                          ^ d: amphora_rack.ts:47
    return x;
  }
  other(x: Amphora): Amphora {
  //       ^ d: amphora_rack.ts:45
  //                 ^ d: amphora_rack.ts:45
    return x;
  }
}

export class AmphoraMaker<Amphora> {
  make(): Amphora {
  //      ^ d: amphora_rack.ts:60
    return new Amphora();
    //         ^ d: amphora.ts:1
  }
}

export class AmphoraWrapped<
  Amphora,
//^ d: amphora_rack.ts:69
> {
  held: Amphora;
  //    ^ d: amphora_rack.ts:69
  take(x: Amphora): Amphora {
  //      ^ d: amphora_rack.ts:69
    return x;
  }
}

export function amphoraWrappedOne<
  Amphora,
>(x: Amphora): Amphora {
//   ^ d: amphora_rack.ts:81
//             ^ d: amphora_rack.ts:81
  return x;
}

export type AmphoraWrappedPair<
  Amphora,
> = [Amphora, Amphora];
//   ^ d: amphora_rack.ts:89
//            ^ d: amphora_rack.ts:89

export class AmphoraWrappedHost {
  own<
    Amphora,
  >(x: Amphora): Amphora {
  //   ^ d: amphora_rack.ts:96
  //             ^ d: amphora_rack.ts:96
    return x;
  }
  plain(x: Amphora): Amphora {
  //       ^ d: amphora.ts:1
    return x;
  }
}

export interface AmphoraWrappedBound<
  K,
  T extends Amphora,
//          ^ d: amphora.ts:1
> {
  get(k: K): T;
  //     ^ d: amphora_rack.ts:109
  //         ^ d: amphora_rack.ts:110
}

export function amphoraWrappedArgs<Amphora>(
  x: Amphora,
  // ^ d: amphora_rack.ts:118
  y: number,
): Amphora {
// ^ d: amphora_rack.ts:118
  return x;
}

export function amphoraWrappedCall(x: Amphora): Amphora {
//                                    ^ d: amphora.ts:1
  const y = amphoraWrappedOne<
    Amphora
  //^ d: amphora.ts:1
  >(x);
  return y;
}

export const amphoraWrappedArrow = <
  Amphora,
>(x: Amphora): Amphora => x;
//   ^ d: amphora_rack.ts:137

export function amphoraCrossBound<
  K extends keyof Amphora,
  Amphora,
>(obj: Amphora, k: K): Amphora {
//     ^ d: amphora_rack.ts:143
//                 ^ d: amphora_rack.ts:142
  return obj;
}

export function amphoraArgWraps<Amphora>(x: Amphora, y: Array<
  number
>): Amphora {
//  ^ d: amphora_rack.ts:150
  return x;
}

export function amphoraArgWrapsA<A>(x: A, y: Array<
  number
>): A {
//  ^ d: amphora_rack.ts:157
  return x;
}

export class AmphoraArgWrapsHost {
  own<Amphora>(x: Amphora, y: Array<
    number
  >): Amphora {
  //  ^ d: amphora_rack.ts:165
    return x;
  }
  ownA<A>(x: A, y: Array<
    number
  >): A {
  //  ^ d: amphora_rack.ts:171
    return x;
  }
}

export class AmphoraClosesOnLast<
  K,
  Amphora> {
  held: Amphora;
  //    ^ d: amphora_rack.ts:181
  key: K;
  //   ^ d: amphora_rack.ts:180
}

export function amphoraClosesOnLast<
  Amphora>(x: Amphora): Amphora {
  //          ^ d: amphora_rack.ts:189
  //                    ^ d: amphora_rack.ts:189
  return x;
}

export function amphoraOutside(x: Amphora): Amphora {
//                                ^ d: amphora.ts:1
  return x;
}
