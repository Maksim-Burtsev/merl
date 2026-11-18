<?php

namespace Shop\Storage;

#[\AllowDynamicProperties] final class Pallet extends Skid
{
    public function tilt(): int
    {
        return $this->shrink();
        //            ^ d: src/Storage/Skid.php:7
        //            status: via $this: Pallet
    }
}

function lean(): int
{
    $p = new Pallet();
    return $p->tilt() + $p->shrink();
    //         ^ d: src/Storage/Pallet.php:7
    //         status: via $p: Pallet
    //                      ^ d: src/Storage/Skid.php:7
    //                      status: via $p: Pallet
}

/**
 * @method int glide()
 */
#[Probe] final class Dolly extends \Symfony\Component\Console\Command\Command
{
    public function roll(): int
    {
        return $this->tilt();
        //            ^ d: none
    }
}

function push(): int
{
    $d = new Dolly();
    return $d->glide();
    //         ^ d: src/Storage/Pallet.php:26
    //         status: via $d: Dolly
}
