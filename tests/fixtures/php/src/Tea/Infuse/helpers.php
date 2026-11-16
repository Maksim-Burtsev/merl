<?php

namespace Shop\Tea\Infuse;

if (!function_exists('Shop\Tea\Infuse\warm_cup')) {
    function warm_cup(): int
    {
        return 1;
    }
}

function poured(array $cups): \ArrayObject
{
    $out = new \ArrayObject();
    foreach ($cups as $c) {
        $out->append($c);
    }
    echo $out->count();
    //          ^ d: none
    return $out;
}
