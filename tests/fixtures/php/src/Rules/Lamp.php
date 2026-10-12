<?php

namespace Shop\Rules;

class Lamp
{
    public static ?Gauge $lit = null;

    public function glow(): string
    {
        $lit = new Frame();
        return self::$lit->shine() . $lit->shine();
        //                 ^ d: picker src/Rules/Doubled.php:7, src/Rules/DoubledAgain.php:7, src/Rules/Frame.php:7
    }
}
