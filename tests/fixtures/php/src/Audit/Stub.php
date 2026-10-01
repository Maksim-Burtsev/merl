<?php

namespace Shop\Audit;

class Stub
{
    public const SOURCE = <<<'PHP'
namespace Shop\Ledger;
PHP;

    public function open(): int
    {
        return Tally::OPEN;
        //     ^ d: src/Audit/Tally.php:5
    }
}
