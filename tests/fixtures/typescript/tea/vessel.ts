export interface Vessel {
  volume: number;
}

export interface Vessel {
  pour(cups: number): number;
}

export type Blend = { leaves: number; water: number };
export const Blend = {
  mix(leaves: number): Blend {
    return { leaves, water: leaves * 100 };
  },
};

export enum Leaf {
  Green,
  Black = 4,
}

export namespace Leaf {
  export function parse(name: string): Leaf {
    return name === "black" ? Leaf.Black : Leaf.Green;
  }
}

export namespace Caddy {
  export const size = 2;
}

export namespace Caddy {
  export function fill(): number {
    return size * 2;
    //     ^ d: tea/vessel.ts:28
  }
}
