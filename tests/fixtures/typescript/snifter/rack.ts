export class SnifterRack {
  hangGlass(a: string): string;
  hangGlass(a: any): any { return a; }
}

export function rackUp(r: SnifterRack): string {
  return r.hangGlass("x");
  //       ^ d: snifter/rack.ts:2
}
