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
