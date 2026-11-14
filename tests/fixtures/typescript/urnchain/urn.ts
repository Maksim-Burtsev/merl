export class Urn {
  boil(): number {
    return 1;
  }

  static builder(): UrnBuilder {
    return new UrnBuilder();
  }
}

export class UrnBuilder {
  size(n: number): UrnBuilder {
    return this;
  }

  build(): Urn {
    return new Urn();
  }
}

export class Counter {
  urn(): Urn {
    return new Urn();
  }

  loose() {
    return (this as any).u;
  }
}

export class Shop {
  counter(): Counter {
    return new Counter();
  }

  mystery() {
    return (this as any).k;
  }
}

export class Ewer {
  boil(): number {
    return 3;
  }

  size(n: number): this {
    return this;
  }

  named(s: string): Ewer {
    return this;
  }
}

export class TallEwer extends Ewer {
  boil(): number {
    return 4;
  }
}

export class Stall {
  maybeCounter(): Counter | undefined {
    return undefined;
  }

  maybe(): Ewer | undefined {
    return undefined;
  }
}
