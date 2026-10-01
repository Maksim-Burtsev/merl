<?php namespace Shop\Ledger;

class Book
{
    public function open(): int
    {
        return Tally::OPEN;
        //     ^ d: src/Ledger/Tally.php:3
        //            ^ d: src/Ledger/Tally.php:5
    }
}
