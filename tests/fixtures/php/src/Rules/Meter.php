<?php

namespace Shop\Rules;

class Meter extends Gauge
{
    public function count(): int
    {
        return self::$reads;
        //            ^ d: src/Rules/Gauge.php:7
    }
}
