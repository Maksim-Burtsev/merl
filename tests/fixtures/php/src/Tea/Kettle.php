<?php

namespace Shop\Tea;

class Kettle extends Vessel
{
    public const SIZE = 3;

    public int $lid = 0;

    public function pour(int $cups): int
    {
        return $cups;
    }

    public function decant(): int
    {
        return $this->pour(1) + parent::rinse() + static::SIZE;
        //            ^ d: src/Tea/Kettle.php:11
        //                              ^ d: src/Tea/Vessel.php:14
        //                                                ^ d: src/Tea/Kettle.php:7
    }
}
