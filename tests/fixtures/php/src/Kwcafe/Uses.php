<?php

namespace Shop\Kwcafe;

function serve(): int
{
    $mugful = 5;
    $pot = new Kwpot();
    kwbrew(mugful: $mugful);
    //     ^ d: src/Kwcafe/brewer.php:7
    $pot->kwbrew(mugful: 4);
    //           ^ d: src/Kwcafe/Kwpot.php:9
    return kwbrew($mugful);
}
