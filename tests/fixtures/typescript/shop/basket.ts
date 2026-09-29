import Till, { Tariff, discount } from "./pricing";
import * as pricing from "./pricing";
import { Coupon as Voucher, Currency, Rates } from "@/shop/pricing";
import type { Money, Priced } from "./pricing";
import { Courier, weigh } from "./warehouse";

const WEIGHT_LIMIT = 30;

type Props = { weight: number };

export class Basket {
  private coupon: Voucher = new Voucher();
  //                            ^ d: shop/pricing.ts:18
  owner = "guest";

  constructor(private tariff: Tariff) {}
  //                          ^ d: shop/pricing.ts:9

  gross(): Money {
    //     ^ d: shop/pricing.ts:35
    return discount(this.tariff.rate());
    //     ^ d: shop/pricing.ts:27
    //                   ^ d: shop/basket.ts:16
    //                          ^ d: shop/pricing.ts:10
  }

  bonus(): number {
    return this.coupon.rate() + this.gross();
    //                 ^ d: shop/pricing.ts:19
    //                               ^ d: shop/basket.ts:19
  }
}

export class GiftBasket extends Basket {
  gross(): Money {
    return super.gross() + this.owner.length;
    //           ^ d: shop/basket.ts:19
    //                          ^ d: shop/basket.ts:14
  }
}

export function describeAny(offer: { describe(): string } | Tariff | Voucher): string {
  return (offer as Tariff | Voucher).describe();
  //                                 ^ d: picker shop/pricing.ts:13, shop/pricing.ts:22
}

export function viaNamespace(): number {
  return pricing.discount(3) + Rates.base;
  //             ^ d: shop/pricing.ts:27
  //                                 ^ d: shop/pricing.ts:43
}

export function till(): boolean {
  return new Till().open();
  //         ^ d: shop/pricing.ts:46
}

export function restock(discount: number): number {
  return discount + WEIGHT_LIMIT;
  //     ^ d: shop/basket.ts:58
  //                ^ d: shop/basket.ts:7
}

export function overweight(grams: number): boolean {
  const WEIGHT_LIMIT = 50;
  return weigh(grams) > WEIGHT_LIMIT;
  //     ^ d: shop/warehouse.ts:11
  //                    ^ d: shop/basket.ts:65
}

export function dispatch(): number {
  const courier = new Courier("post");
  //                  ^ d: shop/warehouse.ts:6
  return courier.name.length + courier.pack();
  //             ^ d: shop/warehouse.ts:7
  //                                    ^ d: shop/warehouse.ts:8
}

export function priceOf(p: Priced): number {
  return p.price();
  //       ^ d: shop/pricing.ts:32
}

export function euro(): Currency {
  return Currency.Euro;
  //     ^ d: shop/pricing.ts:37
  //              ^ d: none; want shop/pricing.ts:38 (#341)
}

export function heavy(p: Props): number {
  //                     ^ d: picker shop/basket.ts:9, shop/props.ts:1; want shop/basket.ts:9 (#337)
  return schedule({ weight: p.weight });
  //                ^ d: none; want shop/basket.ts:96 (#315)
}

function schedule(o: { weight: number }): number {
  return Object.values(o).length;
  //            ^ d: shop/props.ts:8; want none (#339)
}
