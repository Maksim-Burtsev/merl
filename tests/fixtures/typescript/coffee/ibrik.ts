export class Ibrik {
  static kind(): string {
    return "k";
  }
  kind(): string {
    return "i";
  }
  static tagline(): string {
    return this.kind();
    //          ^ d: picker coffee/ibrik.ts:2, coffee/ibrik.ts:5
  }
}
