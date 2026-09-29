<?php

namespace Shop\Warehouse;

function weigh(int $grams): int
{
    return intdiv($grams, 1000);
}
