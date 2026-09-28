<?php

namespace Shop\Pricing;

use Shop\Pricing\Stamps as Punches;

class Voucher extends Tariff implements Priced
//                    ^ d: src/Pricing/Tariff.php:9
//                                      ^ d: src/Pricing/Priced.php:5
{
    use Stamps;
    //  ^ d: src/Pricing/Stamps.php:5

    const BONUS = 1;

    public function rate(): int
    {
        return parent::rate() + static::BONUS;
        //             ^ d: picker src/Pricing/Voucher.php:16, src/Pricing/Coupon.php:15, src/Pricing/Priced.php:7, src/Pricing/Tariff.php:19; want src/Pricing/Tariff.php:19 (#356)
        //                              ^ d: src/Pricing/Voucher.php:14
    }

    public function cap(?int $cap): int
    {
        $cap ??= discount(total: 5);
        //                ^ d: picker src/Basket.php:91, src/Basket.php:92; want src/Pricing/functions.php:8 (#316)
        return $cap;
        //      ^ d: picker src/Pricing/Voucher.php:23, src/Pricing/Voucher.php:25; want src/Pricing/Voucher.php:23 (#464)
    }

    public function punches(): array
    {
        $this->stamps = 1;
        $map = ['stamps' => $this->stamps];
        //                         ^ d: src/Pricing/Stamps.php:7
        return $map;
    }
}
