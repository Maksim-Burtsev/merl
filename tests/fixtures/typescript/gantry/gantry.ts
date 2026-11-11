export class Gantry<T> {
  constructor(public load: T) {}

  first(): T {
    return this.load;
  }

  grab<T>(x: T): T {
    return x;
  }
}

export class Crate {
  fill(): void {}
}

export class Barrel {
  fill(): void {}
}

export class Pair<K, V> {
  right(): V {
    return undefined as V;
  }
}

export class Twin extends Pair<Barrel, Crate> {}

export function paired(tw: Twin): void {
  tw.right().fill();
  //         ^ d: gantry/gantry.ts:14
}
