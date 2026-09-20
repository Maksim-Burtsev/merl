import { HatchBase, TrunkBase } from "./aliased";

export class Boot extends TrunkBase {
  lock(): void {
    super.lock();
    this.audit.deleteUser(1);
  }
}

export function shut(trunk: TrunkBase, hatch: HatchBase): void {
  trunk.lock();
  void hatch.deleteUser(2);
}
