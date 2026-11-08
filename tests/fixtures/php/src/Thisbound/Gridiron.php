<?php

namespace Shop\Thisbound;

class Gridiron
{
    public function sear(): int
    {
        return 2;
    }

    public function scoped(array $items): array
    {
        $a = array_map(function ($i) { return $this->sear(); }, $items);
        //                                           ^ d: src/Thisbound/Gridiron.php:7
        $b = array_map(fn ($i) => $this->sear(), $items);
        //                               ^ d: src/Thisbound/Gridiron.php:7
        return [$a, $b];
    }
}
