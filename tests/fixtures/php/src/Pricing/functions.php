<?php

namespace Shop\Pricing;

define('CURRENCY', 'EUR');
const WEIGHT_LIMIT = 30;

function discount(int $total): int
{
    return min($total, Tariff::RATE_CAP) - 1;
}

function &settle(array &$rows): array
{
    return $rows;
}

$banner = <<<EOT
function weigh(int \$grams): int
class Basket
EOT;

$note = <<<'EOT'
function settle(): int
EOT;
