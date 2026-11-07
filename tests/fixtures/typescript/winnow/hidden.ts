import { Winnow } from "./barn";

export function local(): number {
  class Winnow {
    sieveChaff(id: number): number {
      return id;
    }
  }
  const r = new Winnow();
  return r.sieveChaff(1);
  //       ^ d: winnow/hidden.ts:5
}

export function assigned(): number {
  const Winnow = class {
    sieveChaff(id: number): number {
      return id;
    }
  };
  const r = new Winnow();
  return r.sieveChaff(2);
  //       ^ d: !jump
}

export function param(Winnow: new () => { sieveChaff(id: number): string }): string {
  const r = new Winnow();
  return r.sieveChaff(3);
  //       ^ d: !jump
}

export function annotated(): number {
  class Winnow {
    sieveChaff(id: number): number {
      return id;
    }
  }
  const r: Winnow = new Winnow();
  return r.sieveChaff(4) + new Winnow().sieveChaff(5);
  //       ^ d: winnow/hidden.ts:33
  //                                    ^ d: winnow/hidden.ts:33
}

export function imported(): Promise<void> {
  const r = new Winnow();
  return r.sieveChaff(6);
  //       ^ d: winnow/barn.ts:2
}

export async function loaded(): Promise<void> {
  const { Winnow } = await import("./barn");
  return new Winnow().sieveChaff(7);
  //                  ^ d: winnow/barn.ts:2
}
