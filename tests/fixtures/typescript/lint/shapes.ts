export class Painter {
  render(
    line: string,
  ): string {
    return line;
  }

  paint({ a, b }: Brush) {
    return a + b;
  }
}

type Brush = { a: number; b: number };

// A parameter's type literal holds `;`: the header is still a method's (#528).
export class Easel {
  stroke({ a }: { a: number; b: number }) {
    return a;
  }
}
