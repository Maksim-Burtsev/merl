/*
 * The shop of #307: every `d` case carries its answer in a comment under it.
 * A block comment that reads like code declares nothing:
 * export class Tariff {
 */

export const RATE_CAP = 100;

export class Tariff {
  rate(): number {
    return 1;
  }
  describe(): string {
    return "tariff";
  }
}

export class Coupon {
  rate(): number {
    return 2;
  }
  describe(): string {
    return "coupon";
  }
}

export function discount(total: number): number {
  return Math.min(total - 1, RATE_CAP);
}

export interface Priced {
  price(): number;
}

export type Money = number;

export enum Currency {
  Euro = "EUR",
  Dollar = "USD",
}

export namespace Rates {
  export const base = 1;
}

export default class Till {
  open(): boolean {
    return true;
  }
}
