import { Cup } from "./cup";

declare module "./cup" {
  interface Cup {
    buff(): void;
  }
}

export function buffed(cup: Cup) {
  cup.brim();
  //  ^ d: rinse/cup.ts:6
  cup.buff();
  //  ^ d: rinse/buff.ts:5
}
