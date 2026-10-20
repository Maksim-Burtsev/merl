<?php

namespace Shop\Tea\Infuse;

enum Flush: string
{
    case First = 'first';
    case Second = 'second';

    public function label(): string
    {
        return $this->value;
    }
}
