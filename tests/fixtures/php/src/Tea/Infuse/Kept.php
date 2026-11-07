<?php

namespace Shop\Tea\Infuse;

/*
use Shop\Tea\Stale\Kettle;
use function Shop\Tea\Stale\steep;
*/
use Shop\Tea\Kettle;
use function Shop\Tea\steep;

final class Kept
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
