import { Filter, makeStrainer, mesh } from "./sieves";
import { trim as clip, Basket, Strainer } from "./strainer";
import type { Sieve } from "./strainer";

export function infuse(words: string[], basket: Basket): number {
  const line = words.join(" ").trim();
  //                           ^ d: none
  const parts = line.split(",");
  //                 ^ d: none
  const filter = new Filter();
  //                 ^ d: tea/strainer.ts:11
  const holes = filter
    .shake()
   //^ d: tea/strainer.ts:20
    .tilt()
   //^ d: tea/strainer.ts:24
    .basket()
   //^ d: tea/strainer.ts:28
    .split(parts.length);
   //^ d: tea/strainer.ts:37
  const fine = makeStrainer().basket().trim;
  //                          ^ d: tea/strainer.ts:28
  //                                   ^ d: tea/strainer.ts:34
  return holes + fine + filter.rinse() + filter.holes + basket.trim;
  //                           ^ d: tea/strainer.ts:16
  //                                            ^ d: tea/strainer.ts:2
  //                                                           ^ d: tea/strainer.ts:34
}

export function rinseAll(basket: Basket, baskets: Basket[]): number {
  let total = 0;
  for (const basket of baskets) {
    total += basket.split(1);
    //       ^ d: tea/infusion.ts:32
  }
  return total + basket.trim;
  //             ^ d: tea/infusion.ts:30
}

export function deepest(basket: Basket): Basket {
  while (basket.next) basket = basket.next;
  //                           ^ d: tea/infusion.ts:40
  //                                  ^ d: tea/strainer.ts:35
  return basket;
}

export function measure(words: string[]): number[] {
  return words.map((clip) => clip.length + inner());
  //                         ^ d: tea/infusion.ts:48
  //                                       ^ d: tea/infusion.ts:52

  function inner(): number {
    return clip("x").length;
    //     ^ d: tea/strainer.ts:54
  }
}

export function unpack(strainer: mesh.Strainer): number {
  //                                  ^ d: tea/strainer.ts:11
  const { holes, rinse } = strainer;
  return holes + rinse.call(strainer) + strainer.fineness();
  //     ^ d: tea/infusion.ts:60
  //             ^ d: tea/infusion.ts:60
  //                                             ^ d: tea/strainer.ts:12
}

export function sieveOf(): number {
  const sieve: Sieve = { sift: (grams) => grams };
  return sieve.sift(2);
  //           ^ d: tea/infusion.ts:68
}

export async function late(): Promise<number> {
  const lazy = await import("./strainer");
  return lazy.makeStrainer().fineness();
  //          ^ d: tea/strainer.ts:46
}

export function settled(basket: Basket): Promise<number> {
  return Promise.resolve(basket.then()).then((n) => n);
  //                            ^ d: tea/strainer.ts:41
  //                                    ^ d: none
}

export const recipe = `
export class Strainer {
  shake() { return ${makeStrainer().fineness()}; }
  //                                ^ d: tea/strainer.ts:12
}
`;

// export function makeStrainer(): Strainer {}
export function strain(): Strainer {
  //                      ^ d: tea/strainer.ts:11
  const Cup = class Mug {
    pour(): number {
      return this.size();
      //          ^ d: tea/infusion.ts:101
    }

    size(): number {
      return 1;
    }
  };
  return makeStrainer().shake();
  //     ^ d: tea/strainer.ts:46
  //                    ^ d: tea/strainer.ts:20
}

export function tidy(text: string, cups: Map<string, Basket>): number {
  return text.trim().length + (cups.get("a")?.split(1) ?? 0);
  //          ^ d: none
  //                                ^ d: none
  //                                          ^ d: tea/strainer.ts:37
}

export function besideMeasure(): number {
  if (besideMeasure.length) {
    return inner();
    //     ^ d: !jump
  }
  return 0;
}

export function pot(): number {
  const Pot = class {
    pour(): number {
      return this.size();
      //          ^ d: tea/infusion.ts:132
    }

    size(): number {
      return 2;
    }
  };
  return new Pot().pour();
}

const sprinkle = (n: number) =>
  makeStrainer().shake() + n;
//^ d: tea/strainer.ts:46

if (sprinkle.length)
  inner();
//^ d: !jump

export function hidesMaker(): number {
  function makeStrainer(): number {
    return 1;
  }
  function inner(): number {
    return 2;
  }
  return makeStrainer() + inner();
}
