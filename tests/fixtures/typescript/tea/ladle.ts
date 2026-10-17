import { Pot } from "./index";

class Ladle extends Pot {
  pour(cups: number): number {
    return cups * 2;
  }
}

function topOf<T extends Pot>(pots: T[]): T {
  return pots[0];
}

const ladles: Ladle[] = [];

export function ladleOut(): number {
  const top = topOf(ladles);
  return topOf(ladles).pour(1) + top.pour(2);
  //                   ^ d: picker tea/ladle.ts:4, …
  //                                 ^ d: picker tea/ladle.ts:4, …
}

interface Spout {
  pour(): number;
}

declare function register<S>(make: () => void): void;
declare function spout(): Spout;

register<Spout>(function () {
  const s: Spout = spout();
  s.pour();
  //^ d: tea/ladle.ts:23
});
