import { Cup } from "./cup";

declare module "./cup" {
  interface Cup {
    rinse(): void;
  }
}

Cup.prototype.rinse = function () {};
