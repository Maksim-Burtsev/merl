<?php

namespace Shop\Ledger;

class Journal
{
    public function open(): int
    {
        return Tally::OPEN;
        //     ^ d: src/Ledger/Tally.php:3
    }
}

namespace Shop\Audit;

class Inspection
{
    public function open(): int
    {
        return Tally::OPEN;
        //     ^ d: src/Audit/Tally.php:5
        //            ^ d: src/Audit/Tally.php:7
    }
}

class Recount extends Tally
{
    public function open(): int
    {
        return parent::OPEN;
        //             ^ d: src/Audit/Tally.php:7
    }

    public function again(): int
    {
        return static::OPEN;
        //             ^ d: src/Audit/Tally.php:7
    }

    public function measure(Tally $t): int
    {
        return $t->sum();
        //         ^ d: src/Audit/Tally.php:9
    }
}
