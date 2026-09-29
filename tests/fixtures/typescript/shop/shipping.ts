import { Courier } from "./index";

export function ship(): string {
  return new Courier("van").name;
  //         ^ d: shop/warehouse.ts:6
}

export function bare(total: number): number {
  return discount(total);
  //     ^ d: shop/pricing.ts:27
  //     status: by name, 1 match
}
