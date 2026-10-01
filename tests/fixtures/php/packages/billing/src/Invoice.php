<?php

namespace Billing;

use Shop\Audit\Tally as Audited;

class Invoice
{
    public function open(): int
    {
        return Audited::OPEN + \Shop\Ledger\Tally::OPEN + Tally::OPEN;
        //     ^ d: src/Audit/Tally.php:5
        //              ^ d: src/Audit/Tally.php:7
        //                           ^ d: picker src/Audit/Blocks.php:3, src/Ledger/Book.php:1, src/Ledger/Tally.php:1
        //                                  ^ d: src/Ledger/Tally.php:3
        //                                         ^ d: src/Ledger/Tally.php:5
        //                                                ^ d: packages/billing/src/Tally.php:5
    }
}
