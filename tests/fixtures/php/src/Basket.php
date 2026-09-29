<?php

namespace Shop;

use Shop\Pricing\Coupon;
use Shop\Pricing\Offer;
use Shop\Pricing\Tariff as Plan;
use Shop\Warehouse\Courier;
use function Shop\Warehouse\weigh;
use function Shop\Pricing\discount;
use const Shop\Pricing\WEIGHT_LIMIT;

#[\Attribute]
final class Basket
{
    public const MAX = 10;
    private array $rows = [];

    public function __construct(
        private Plan $tariff,
        //      ^ d: none; want src/Pricing/Tariff.php:9 (#351)
        private Coupon $coupon,
        //      ^ d: src/Pricing/Coupon.php:9
        public string $owner = '',
    ) {
    }

    public function gross(): int
    {
        return discount($this->tariff->rate());
        //     ^ d: src/Pricing/functions.php:8
        //                     ^ d: picker src/Basket.php:20, src/Basket.php:115; want src/Basket.php:20 (#356)
        //                             ^ d: picker src/Pricing/Coupon.php:15, src/Pricing/Priced.php:7, src/Pricing/Tariff.php:19, src/Pricing/Voucher.php:16; want src/Pricing/Tariff.php:19 (#361)
    }

    public function bonus(): int
    {
        return $this->coupon->rate() + $this->gross();
        //                    ^ d: picker src/Pricing/Coupon.php:15, src/Pricing/Priced.php:7, src/Pricing/Tariff.php:19, src/Pricing/Voucher.php:16; want src/Pricing/Coupon.php:15 (#361)
        //                                    ^ d: src/Basket.php:28
    }

    public function describeAny(Plan $t, Coupon $c): string
    {
        return $t->describe() . $c->describe();
        //         ^ d: picker src/Pricing/Coupon.php:20, src/Pricing/Tariff.php:25; want src/Pricing/Tariff.php:25 (#361)
        //                          ^ d: picker src/Pricing/Coupon.php:20, src/Pricing/Tariff.php:25; want src/Pricing/Coupon.php:20 (#361)
    }

    public function restock(int $weigh): int
    {
        return $weigh + WEIGHT_LIMIT + self::MAX;
        //      ^ d: src/Warehouse/functions.php:5; want src/Basket.php:50 (#464)
        //              ^ d: src/Pricing/functions.php:6
        //                                   ^ d: src/Basket.php:16
    }

    public function overweight(int $grams): bool
    {
        $rows = [$grams];
        $rows[] = weigh($grams);
        //        ^ d: src/Warehouse/functions.php:5
        return count($rows) > count($this->rows);
        //            ^ d: picker src/Basket.php:17, src/Basket.php:60; want src/Basket.php:60 (#464)
        //                                 ^ d: picker src/Basket.php:17, src/Basket.php:60; want src/Basket.php:17 (#348)
    }

    public function dispatch(): string
    {
        $courier = new Courier('post');
        //             ^ d: src/Warehouse/Courier.php:5
        return $courier->name . Courier::depot();
        //               ^ d: src/Warehouse/Courier.php:7
        //                               ^ d: src/Warehouse/Courier.php:11
    }

    public function offer(Offer $o): int
    {
        switch ($o) {
            case Offer::Plain:
                //      ^ d: src/Pricing/Offer.php:7
                return 0;
            default:
                return Offer::DEFAULT === $o ? 1 : 2;
                //            ^ d: src/Pricing/Offer.php:10
        }
    }

    public function money(): string
    {
        $total = 0;
        $total += Plan::RATE_CAP;
        //              ^ d: src/Pricing/Tariff.php:11
        $label = CURRENCY;
        //       ^ d: src/Pricing/functions.php:5
        $label .= (string) $total;
        //                  ^ d: picker src/Basket.php:91, src/Basket.php:92
        return $label;
    }

    public function pay(): array
    {
        return \Shop\Pricing\settle($this->rows);
        //                   ^ d: src/Pricing/functions.php:13
    }

    public function coupon(): string
    {
        return $this->coupon->code . $this->coupon->bonus() . $this->coupon->stamps;
        //                    ^ d: none; want src/Pricing/Coupon.php:6 (#344)
        //                                          ^ d: src/Basket.php:36; want src/Pricing/Coupon.php:7 (#344)
        //                                                                   ^ d: src/Pricing/Stamps.php:7
    }

    public function tariff(): int
    {
        $plan = new Plan(base: 3);
        //               ^ d: none; want src/Pricing/Tariff.php:15 (#351)
        return $plan->base;
        //            ^ d: src/Pricing/Tariff.php:15
    }

    public function floor(): int
    {
        return (new Plan())->rate();
    }
}

function labels(): array
{
    return array_map(callback: null, array: []);
    //               ^ d: none
    //               status: callback: argument label
}
