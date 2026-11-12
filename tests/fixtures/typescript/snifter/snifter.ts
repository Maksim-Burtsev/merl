export class Snifter {
  nose(): string { return "oak"; }
  breathe(seconds: number): number { return seconds * 2; }
  async swirl(): Promise<string> { return `${this.nose()}`; }
  //                                              ^ d: snifter/snifter.ts:2
}
