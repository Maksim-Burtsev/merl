import { Snifter } from "./snifter";

class Flight {
  nose(): string { return "flight"; }
}

export class SnifterTray {
  raiseTray() { return new Snifter(); }
  presentTray() {
    return new Flight();
  }
}

export function serveTray(): string {
  const tray = new SnifterTray();
  const raised = tray.raiseTray();
  return raised.nose();
  //            ^ d: snifter/snifter.ts:2
}
