<?php

namespace Shop\Warehouse;

class Quay
{
    public function unload($ship, $crane, ?Dock $dock): string
    {
        $ship->manifest('a', 'b');
        //     ^ d: none
        $ship->moor('a');
        //     ^ d: src/Warehouse/Dock.php:15
        $dock?->cargo('x');
        //      ^ d: none
        $crane
            ->moor('b')
            //^ d: src/Warehouse/Dock.php:15
            ?->berth;
            // ^ d: none
        return $crane->cargo . $dock->berth.hoist();
        //             ^ d: src/Warehouse/Dock.php:7
        //                                  ^ d: src/Warehouse/Dock.php:20
    }
}
