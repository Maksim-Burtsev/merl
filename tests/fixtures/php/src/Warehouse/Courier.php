<?php

namespace Shop\Warehouse;

class Courier
{
    public function __construct(public string $name)
    {
    }

    public static function depot(): bool
    {
        return true;
    }
}
