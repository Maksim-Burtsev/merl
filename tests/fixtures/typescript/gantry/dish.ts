export class T {
  scour(): void {}
}

export class Tub {
  scour(): void {}
}

export class Dish<T> {
  constructor(public held: T) {}

  lid(): T {
    return this.held;
  }

  pick<T>(x: T): T {
    return x;
  }
}

export class Mug extends Dish<Tub[]> {}

export function lift(d: Dish<Tub>, m: Mug, j: Tub): void {
  d.lid().scour();
  //      ^ d: picker gantry/dish.ts:2, gantry/dish.ts:6
  m.lid().scour();
  //      ^ d: picker gantry/dish.ts:2, gantry/dish.ts:6
  d.pick(j).scour();
  //        ^ d: picker gantry/dish.ts:2, gantry/dish.ts:6
}

export function make(): Dish<Tub> {
  return new Dish(new Tub());
}

export function chained(): void {
  make().lid().scour();
  //           ^ d: picker gantry/dish.ts:2, gantry/dish.ts:6
}
