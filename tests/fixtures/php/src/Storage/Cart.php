<?php

namespace Shop\Storage;

interface HasLevel { const LEVEL_TOP = 1; }
final class Cart extends \Symfony\Component\Console\Command\Command implements HasLevel
{
    public function run(): int { return self::LEVEL_TOP; }
    //                                        ^ d: none; want src/Storage/Cart.php:5 (#1588)
    public function top(): int
    {
        return static::LEVEL_TOP + Cart::LEVEL_TOP;
        //             ^ d: none; want src/Storage/Cart.php:5 (#1588)
        //                               ^ d: src/Storage/Cart.php:5
    }
}
