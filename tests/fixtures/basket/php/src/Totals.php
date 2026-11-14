<?php

namespace App;

function doubled(array $prices): \ArrayObject
{
    $out = new \ArrayObject();
    foreach ($prices as $p) {
        $out->append($p * 2);
        //     ^ d: none
    }
    echo $out->count();
    //         ^ d: none
    return $out;
}
