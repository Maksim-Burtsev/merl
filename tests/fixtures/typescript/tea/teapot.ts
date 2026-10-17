import Kettle from "./kettle";
import type { Vessel } from "./vessel";

export class Lid {
  lift(): boolean {
    return true;
  }
}

export class Teapot implements Vessel {
  volume = 4;
  heat = 70;
  lid?: Lid;

  pour(cups: number): number {
    return cups;
  }

  serve(): number {
    const viaArrow = () => this.heat;
    //                          ^ d: tea/teapot.ts:12
    const viaFunction = function (this: Kettle): number {
      return this.heat;
      //          ^ d: tea/kettle.ts:2
    };
    return viaArrow() + viaFunction.call(new Kettle());
    //     ^ d: tea/teapot.ts:20
    //                                       ^ d: tea/kettle.ts:1
  }
}

export function warm(this: Kettle): number {
  const inner = () => this.heat;
  //                       ^ d: tea/kettle.ts:2
  return inner();
}
