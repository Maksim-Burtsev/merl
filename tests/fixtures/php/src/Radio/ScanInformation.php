<?php

namespace Shop\Radio;

class ScanInformation
{
    public function toArray(): array
    {
        return [];
    }

    public static function make(): self
    {
        return new self();
    }

    public static function fresh(): static
    {
        return new static();
    }
}
