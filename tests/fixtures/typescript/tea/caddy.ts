import { Basket } from "./strainer";

export class ReadonlySet {
  keys(): number {
    return 0;
  }
}

export const Array = {
  from(cups: number): Basket {
    return new Basket();
  },
};

export function count(jars: ReadonlySet, cups: number): number {
  return jars.keys() + Array.from(cups).split(1);
  //          ^ d: tea/caddy.ts:4
  //                                    ^ d: tea/strainer.ts:37
}

