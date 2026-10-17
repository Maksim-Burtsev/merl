<?php

namespace Shop\Tea;

function steep(int $minutes): int
{
    return $minutes * 2;
}

function pour(Kettle $kettle): int
{
    return $kettle->pour(1);
    //              ^ d: src/Tea/Kettle.php:11
}

const STRENGTH = 5;

$recipe = <<<TXT
function steep(int \$minutes): int
class Kettle
TXT;

$label = 'function pour(Kettle $kettle): int';
