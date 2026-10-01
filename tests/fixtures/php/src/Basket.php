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
        //      ^ d: src/Pricing/Tariff.php:9
        private Coupon $coupon,
        //      ^ d: src/Pricing/Coupon.php:9
        public string $owner = '',
    ) {
    }

    public function gross(): int
    {
        return discount($this->tariff->rate());
        //     ^ d: src/Pricing/functions.php:8
        //                     ^ d: src/Basket.php:20
        //                             ^ d: src/Pricing/Tariff.php:19
    }

    public function bonus(): int
    {
        return $this->coupon->rate() + $this->gross();
        //                    ^ d: src/Pricing/Coupon.php:15
        //                                    ^ d: src/Basket.php:28
    }

    public function describeAny(Plan $t, Coupon $c): string
    {
        return $t->describe() . $c->describe();
        //         ^ d: src/Pricing/Tariff.php:25
        //                          ^ d: src/Pricing/Coupon.php:20
    }

    public function restock(int $weigh): int
    {
        return $weigh + WEIGHT_LIMIT + self::MAX;
        //      ^ d: src/Basket.php:50
        //              ^ d: src/Pricing/functions.php:6
        //                                   ^ d: src/Basket.php:16
    }

    public function overweight(int $grams): bool
    {
        $rows = [$grams];
        $rows[] = weigh($grams);
        //        ^ d: src/Warehouse/functions.php:5
        return count($rows) > count($this->rows);
        //            ^ d: src/Basket.php:60
        //                                 ^ d: src/Basket.php:17
    }

    public function dispatch(): string
    {
        $courier = new Courier('post');
        //             ^ d: src/Warehouse/Courier.php:5
        return $courier->name . Courier::depot();
        //               ^ d: src/Warehouse/Courier.php:7
        //                               ^ d: src/Warehouse/Courier.php:11
        //                                status: via import src/Warehouse/Courier.php
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
        //                  ^ d: src/Basket.php:92
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
        //                    ^ d: src/Pricing/Coupon.php:6
        //                                          ^ d: src/Pricing/Coupon.php:7
        //                                                                   ^ d: src/Pricing/Stamps.php:7
    }

    public function tariff(): int
    {
        $plan = new Plan(base: 3);
        //               ^ d: src/Pricing/Tariff.php:15
        //          ^ d: src/Pricing/Tariff.php:9
        //           status: via import src/Pricing/Tariff.php
        return $plan->base;
        //            ^ d: src/Pricing/Tariff.php:15
    }

    public function floor(): int
    {
        return (new Plan())->rate();
    }

    public function ledger(array $entries): array
    {
        $tally = 0;
        foreach ($entries as $slot => $entry) {
        //        ^ d: src/Basket.php:131
            $tally += $entry;
            //         ^ d: src/Basket.php:134
          // ^ d: src/Basket.php:133
        }
        $chooser = function (int $low) use ($tally): int {
            return min($low, $tally);
            //          ^ d: src/Basket.php:140
            //                ^ d: src/Basket.php:140
        };
        $doubled = array_map(fn($entry) => $entry * 2, $entries);
        //                                  ^ d: src/Basket.php:145
        //                                              ^ d: src/Basket.php:131
        [$low, $high] = [0, $tally];
        //                   ^ d: src/Basket.php:133
        try {
            $chooser($low);
          // ^ d: src/Basket.php:140
            //        ^ d: src/Basket.php:148
        } catch (\RuntimeException $failure) {
            return [$failure->getMessage()];
            //       ^ d: src/Basket.php:154
        }
        return [$high, $slot, $doubled, $ghost, "$tally"];
        //       ^ d: src/Basket.php:148
        //              ^ d: src/Basket.php:134
        //                     ^ d: src/Basket.php:145
        //                               ^ d: none
        //                                        ^ d: src/Basket.php:133
    }

    public function chime(): int
    {
        return 1;
    }

    public function rung(): int
    {
        $chime = $this->chime();
        //              ^ d: src/Basket.php:166
        return $chime;
    }
}

function labels(): array
{
    return array_map(callback: null, array: []);
    //               ^ d: none
    //               status: callback: argument label
}
