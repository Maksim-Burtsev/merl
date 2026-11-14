import { Cruet } from "./cruet";

export class Phial implements Cruet {
  uncork(): void {}
  stopper(): number {
    return 2;
  }
}
