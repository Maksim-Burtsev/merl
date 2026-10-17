<?php

namespace Shop\Tea;

class Teapot extends Vessel
{
    use Steeps;
    //  ^ d: src/Tea/Steeps.php:5

    public const SIZE = 4;

    public function pour(int $cups): int
    {
        return $cups * static::SIZE;
        //                     ^ d: src/Tea/Teapot.php:10
    }

    public static function make(): static
    {
        return static::create();
        //             ^ d: src/Tea/Vessel.php:9
    }
}
