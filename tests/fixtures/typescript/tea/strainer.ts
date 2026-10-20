export abstract class Mesh {
  holes = 40;

  rinse(): number {
    return this.holes;
  }

  abstract fineness(): number;
}

export class Strainer extends Mesh {
  fineness(): number {
    return 2;
  }

  override rinse(): number {
    return super.rinse() + 1;
  }

  shake(): Strainer {
    return this;
  }

  tilt(): this {
    return this;
  }

  basket(): Basket {
    return new Basket();
  }
}

export class Basket {
  trim = 1;
  next?: Basket;

  split(cups: number): number {
    return cups;
  }

  then(): number {
    return 0;
  }
}

export function makeStrainer(): Strainer {
  return new Strainer();
}

export interface Sieve {
  sift(grams: number): number;
}

export function trim(text: string): string {
  return text;
}

export function inner(): number {
  return 3;
}
