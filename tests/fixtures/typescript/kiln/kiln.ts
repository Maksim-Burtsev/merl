export class Kiln {
  boil(): number {
    return 1;
  }

  size(n: number): this {
    return this;
  }

  named(s: string): Kiln {
    return this;
  }
}

export class TallKiln extends Kiln {
  boil(): number {
    return 2;
  }
}

export class Yard {
  async find(): Promise<Kiln> {
    return new Kiln();
  }

  fetch(): Promise<Kiln> {
    return Promise.resolve(new Kiln());
  }

  async loose() {
    return (this as any).k;
  }

  async vague(): Promise<any> {
    return (this as any).k;
  }
}

export class Shelf<Kiln> {
  async top(): Promise<Kiln> {
    return (this as any).k;
  }
}
