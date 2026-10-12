<?php

namespace Shop\Rules;

trait Greets
{
    public Frame $frame;

    public function hello(): string
    {
        return 'trait';
    }

    public function wave(): string
    {
        return $this->hello();
        //            ^ d: picker src/Rules/Greeter.php:9, src/Rules/Greeter.php:31
    }

    public function beam(): string
    {
        return $this->frame->shine();
        //                   ^ d: picker src/Rules/Doubled.php:7, src/Rules/DoubledAgain.php:7, src/Rules/Frame.php:7
    }
}

class Greeter
{
    use Greets;

    public function hello(): string
    {
        return 'class';
    }
}
