<?php

namespace Shop\Coupon;

use Shop\Pricing\Coupon;
//               ^ d: src/Pricing/Coupon.php:9
use Vendor\Pricing\Ticket;
//         ^ d: none

/**
 * @property-read int $size
 * @method static self ofSize(int $size)
 */
#[\Attribute]
final class Batch
{
    private const ?string PREFIX = null;
    private const int|float CAP = 5, SPARE = 1;

    /**
     * @param Coupon $c
     * @property int $hidden
     */
    public function fill(Coupon $c): int
    //                   ^ d: src/Pricing/Coupon.php:9
    {
        $fresh = \Shop\Coupon\Batch::ofSize(2);
        //                ^ d: src/Coupon/Batch.php:3
        //                           ^ d: src/Coupon/Batch.php:12
        return self::CAP + self::SPARE + $this->size + $this->hidden + strlen(static::PREFIX);
        //           ^ d: src/Coupon/Batch.php:18
        //                       ^ d: none
        //                                      ^ d: src/Coupon/Batch.php:11
        //                                                    ^ d: none
        //                                                                            ^ d: src/Coupon/Batch.php:17
    }
}
