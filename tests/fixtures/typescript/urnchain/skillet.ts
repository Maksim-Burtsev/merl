export class Skillet {
  boil(): number {
    return 3;
  }

  size(n: number): Skillet {
    return this;
  }

  build(): Skillet {
    return this;
  }

  urn(): Skillet {
    return this;
  }
}
