<?php

namespace Shop\Pricing;

trait Stamps
{
    public int $stamps = 0;

    public function stamp(): int
    {
        return $this->stamps;
    }
}
