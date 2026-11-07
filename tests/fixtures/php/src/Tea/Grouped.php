<?php

namespace Shop\Tea;

/*
use Shop\Tea\Stale\{Kettle};
use function Shop\Tea\Stale\{steep};
*/

final class Grouped
{
    public function boil(): int
    {
        $kettle = new Kettle();
        //            ^ d: src/Tea/Kettle.php:5
        return $kettle->pour(1) + steep(2);
        //              ^ d: src/Tea/Kettle.php:11
        //                        ^ d: src/Tea/brew.php:5
    }
}
