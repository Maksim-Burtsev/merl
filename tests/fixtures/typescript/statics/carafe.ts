export class Carafe {
  static decant(): Carafe {
    return new Carafe();
  }
  decant(n: number): Carafe {
    return this;
  }
  swirl(): void {
  }
  static tip(): Carafe {
    return new Carafe();
  }
  tip(n: number): number {
    return n;
  }
}
