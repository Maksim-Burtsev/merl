<?php

namespace Shop\Warehouse;

use Shop\Pricing\Offer;
//        ^ d: picker src/Pricing/Coupon.php:3, src/Pricing/Offer.php:3, src/Pricing/Priced.php:3, src/Pricing/Stamps.php:3, src/Pricing/Tariff.php:3, src/Pricing/Voucher.php:3, src/Pricing/functions.php:3

class Depot
{
    public function open(Courier $courier): string
    {
        $courier->name = 'depot';
        return $courier->name . Offer::Cut->value;
        //               ^ d: src/Warehouse/Courier.php:7
        //                             ^ d: src/Pricing/Offer.php:8
        //      ^ d: src/Warehouse/Depot.php:10
    }
}
