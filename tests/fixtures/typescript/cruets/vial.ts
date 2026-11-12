import { Cruet } from "./cruet";

export class Vial implements Cruet {
  uncork(): void {}
  stopper(): number {
    return 1;
  }
}

export class Flute extends Vial {
  uncork(): void {}
}
