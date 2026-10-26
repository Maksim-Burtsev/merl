<?php

namespace Shop\Tea;

class Samovar extends Vessel
{
    use Steeps,
        Rinses;

    public function serve(): int
    {
        return $this->rinse() + $this->decant();
        //            ^ d: src/Tea/Rinses.php:7
        //                             ^ d: src/Tea/Steeps.php:7
    }
}
