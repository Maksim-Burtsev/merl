<?php namespace Shop\Ledger;
//              ^ d: picker src/Basket.php:3, src/Paths.php:3

class Book
{
    public function open(): int
    {
        return Tally::OPEN;
        //     ^ d: src/Ledger/Tally.php:3
        //            ^ d: src/Ledger/Tally.php:5
    }
}
