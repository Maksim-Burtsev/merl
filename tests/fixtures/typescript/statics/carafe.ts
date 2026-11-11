export class Carafe {
  static decant(): Carafe {
    return new Carafe();
  }
  decant(n: number): Carafe {
    return this;
  }
  swirl(): void {
  }
}
