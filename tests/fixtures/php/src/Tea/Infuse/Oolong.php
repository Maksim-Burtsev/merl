<?php

namespace Shop\Tea\Infuse;

class Oolong extends Leaf
{
    use Rolled, Roasted {
        Rolled::shape insteadof Roasted;
        Roasted::shape as roastShape;
    }

    public function grade(): string
    {
        return 'oolong';
    }
}
