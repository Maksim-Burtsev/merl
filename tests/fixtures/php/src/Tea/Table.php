<?php

namespace Shop\Tea;

use Shop\Tea\Stale\Kettle as OldKettle;
use function Shop\Tea\Stale\steep;

/*
class Teapot {}
function pour() {}
*/

final class Table
{
    private Kettle $kettle;
    private Teapot $pot;

    public function __construct()
    {
        $this->kettle = new Kettle();
        //                  ^ d: src/Tea/Kettle.php:5
        $this->pot = Teapot::make();
        //           ^ d: src/Tea/Teapot.php:5
        //                   ^ d: src/Tea/Teapot.php:18
    }

    public function serve(Teapot $kettle): int
    {
        return $kettle->pour(1) + $this->kettle->pour(2);
        //      ^ d: src/Tea/Table.php:27
        //              ^ d: src/Tea/Teapot.php:12
        //                               ^ d: src/Tea/Table.php:15
        //                                       ^ d: src/Tea/Kettle.php:11
    }

    public function stale(OldKettle $old): int
    {
        return $old->pour() + (new OldKettle())->pour() + steep(3);
        //           ^ d: src/Tea/Stale/Kettle.php:7
        //                         ^ d: src/Tea/Stale/Kettle.php:5
        //                                       ^ d: picker src/Tea/Kettle.php:11, src/Tea/Stale/Kettle.php:7, src/Tea/Teapot.php:12; want src/Tea/Stale/Kettle.php:7 (#956)
        //                                                ^ d: picker src/Tea/Stale/functions.php:5, src/Tea/brew.php:5; want src/Tea/Stale/functions.php:5 (#955)
    }

    public function fresh(): int
    {
        return \Shop\Tea\steep(3) + pour($this->kettle) + STRENGTH;
        //               ^ d: picker src/Tea/Stale/functions.php:5, src/Tea/brew.php:5; want src/Tea/brew.php:5 (#955)
        //                          ^ d: picker src/Tea/Kettle.php:11, src/Tea/Stale/Kettle.php:7, src/Tea/Teapot.php:12, src/Tea/brew.php:10; want src/Tea/brew.php:10 (#955)
        //                                                ^ d: src/Tea/brew.php:16
    }

    public function swap(): int
    {
        $vessel = new Kettle();
        $vessel = new Teapot();
        return $vessel->pour(1);
        //              ^ d: picker src/Tea/Kettle.php:11, src/Tea/Stale/Kettle.php:7, src/Tea/Teapot.php:12; want src/Tea/Teapot.php:12 (#956)
    }

    public function steeped(): int
    {
        return $this->pot->decant() + $this->kettle->decant();
        //                 ^ d: src/Tea/Steeps.php:7
        //                                           ^ d: src/Tea/Kettle.php:16
    }

    public function sizes(): int
    {
        return Kettle::SIZE + Teapot::SIZE + Vessel::SIZE + OldKettle::class;
        //             ^ d: src/Tea/Kettle.php:7
        //                            ^ d: src/Tea/Teapot.php:10
        //                                           ^ d: src/Tea/Vessel.php:7
        //                                                  ^ d: src/Tea/Stale/Kettle.php:5
    }

    public function closures(): int
    {
        $pot = $this->pot;
        $f = fn (Kettle $pot) => $pot->pour(1);
        //                             ^ d: src/Tea/Kettle.php:11
        $g = function () use ($pot) {
            return $pot->pour(2);
            //           ^ d: picker src/Tea/Kettle.php:11, src/Tea/Stale/Kettle.php:7, src/Tea/Teapot.php:12; want src/Tea/Teapot.php:12 (#956)
        };
        return $f($this->kettle) + $g();
    }

    public function lid(): int
    {
        $lid = $this->kettle->lid;
        //                    ^ d: src/Tea/Kettle.php:9
        return $lid;
        //      ^ d: src/Tea/Table.php:91
    }
}
