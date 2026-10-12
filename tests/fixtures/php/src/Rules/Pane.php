<?php

namespace Shop\Rules;

class Pane extends Frame
{
    use Doubled;

    public function show(): string
    {
        return $this->shine();
        //            ^ d: picker src/Rules/Doubled.php:7, src/Rules/DoubledAgain.php:7, src/Rules/Frame.php:7
    }
}
