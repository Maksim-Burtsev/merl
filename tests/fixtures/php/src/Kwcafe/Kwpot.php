<?php

namespace Shop\Kwcafe;

class Kwpot
{
    public int $mugful = 1;

    public function kwbrew(int $mugful = 2): int
    {
        return $mugful;
    }

    public function mugful(): int
    {
        return 4;
    }
}
