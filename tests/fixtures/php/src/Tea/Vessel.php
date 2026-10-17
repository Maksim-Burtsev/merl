<?php

namespace Shop\Tea;

abstract class Vessel
{
    public const SIZE = 1;

    public static function create(): static
    {
        return new static();
    }

    public function rinse(): int
    {
        return static::SIZE;
        //             ^ d: src/Tea/Vessel.php:7
    }
}
