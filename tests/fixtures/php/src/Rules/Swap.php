<?php

namespace Shop\Rules;

class Swap
{
    public function swap(): string
    {
        $pane = new Frame();
        $pane = new Pane($pane->shine());
        //                      ^ d: src/Rules/Frame.php:7
        return '';
    }

    function lit(): Gauge
    {
        return new Gauge();
    }

    public function lamp(): string
    {
        $l = lit();
        return $l->shine();
        //         ^ d: src/Rules/Frame.php:7
    }
}
