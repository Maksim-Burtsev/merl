import { Snifter } from "./snifter";

class Flute {
  nose(): string { return "pear"; }
}

const glass: Snifter = new Snifter();

export function taste(): number {
  glass.nose();
  //    ^ d: snifter/snifter.ts:2
  new Flute().nose();
  //          ^ d: snifter/tasting.ts:4
  return glass.breathe(3);
  //           ^ d: snifter/snifter.ts:3
}

export function guess(g) {
  return g.nose();
  //       ^ d: picker snifter/snifter.ts:2, snifter/tasting.ts:4
}

export const card = `
class Tumbler {
  breathe(): number { return 1; }
}
`;

export function swirled(): Promise<string> {
  return glass.swirl();
  //           ^ d: snifter/snifter.ts:4
}
