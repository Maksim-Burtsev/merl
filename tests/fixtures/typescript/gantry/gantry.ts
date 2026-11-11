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
