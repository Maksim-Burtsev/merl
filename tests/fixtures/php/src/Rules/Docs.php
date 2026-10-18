<?php

namespace Shop\Rules;

/**
 * @method Docs framed()
 * @property Frame $glass
 */
class Docs
{
    public function made(): Frame
    {
        return new Frame();
    }

    public function show(): string
    {
        $f = $this->framed();
        return $f->shine() . $this->glass->shine();
        //         ^ d: picker src/Rules/Doubled.php:7, src/Rules/DoubledAgain.php:7, src/Rules/Frame.php:7
        //                                 ^ d: src/Rules/Frame.php:7
    }
}
