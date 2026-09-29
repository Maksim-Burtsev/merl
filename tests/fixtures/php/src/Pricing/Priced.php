<?php

namespace Shop\Pricing;

interface Priced
{
    public function rate(): int;
}
