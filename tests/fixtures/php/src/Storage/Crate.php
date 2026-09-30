<?php

namespace Shop\Storage;

use Symfony\Component\Console\Command\Command;

final class Crate extends Command
{
    public function pack(): void
    {
        $this->seal();
        //     ^ d: none
        self::SUCCESS;
        //    ^ d: none
    }
}
