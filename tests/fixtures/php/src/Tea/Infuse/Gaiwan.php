<?php

namespace Shop\Tea\Infuse;

class Gaiwan
{
    public function pour(): int
    {
        return 1;
    }

    public function rinse(): self
    {
        return $this;
    }
}
