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
