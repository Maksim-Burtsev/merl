<?php

namespace Shop\Pricing;

/* A block comment that reads like code declares nothing:
class Coupon {
*/

class Tariff implements Priced
{
    public const RATE_CAP = 100;
    private const int FLOOR = 1;

    public function __construct(
        public readonly int $base = 0,
    ) {
    }

    public function rate(): int
    {
        return self::FLOOR;
        //           ^ d: none; want src/Pricing/Tariff.php:12 (#344)
    }

    public function describe(): string
    {
        return 'tariff';
    }
}
