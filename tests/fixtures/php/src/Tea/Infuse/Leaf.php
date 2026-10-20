<?php

namespace Shop\Tea\Infuse;

class Leaf implements Graded
{
    public function grade(): string
    {
        return 'green';
    }

    public function format(): string
    {
        return $this->grade();
    }

    public function count(): int
    {
        return 1;
    }

    public function withWater(int $ml): static
    {
        return $this;
    }

    public function steep(): Gaiwan
    {
        return new Gaiwan();
    }
}
