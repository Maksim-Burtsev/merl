<?php

namespace Shop\Storage;

#[Probe] final class Boom extends Jib
{
    public function reach(): int
    {
        return $this->jibSwing();
        //            ^ d: src/Storage/Jib.php:7
        //            status: via $this: Boom
    }
}
