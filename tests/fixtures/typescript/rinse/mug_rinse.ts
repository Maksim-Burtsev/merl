import { Mug } from "./cup";

declare module "./cup" {
  interface Mug {
    rinse(): void;
  }
}

Mug.prototype.rinse = function () {};
