<?php

namespace Shop\Pricing;

/**
 * @property string $code
 * @method int bonus()
 */
final class Coupon implements Priced
{
    use Stamps;

    protected string $label = 'coupon';

    public function rate(): int
    {
        return 2 + $this->stamp();
    }

    public function describe(): string
    {
        return $this->label;
    }
}
