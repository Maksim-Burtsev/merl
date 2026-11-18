<?php

namespace Shop\Storage;

use Symfony\Component\Console\Command\Command;
use Symfony\Contracts\Service\ResetInterface;

final class Tote extends Command implements Graded
{
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: src/Storage/Graded.php:7
        return static::TIER;
        //             ^ d: src/Storage/Graded.php:7
        return Tote::TIER;
        //           ^ d: src/Storage/Graded.php:7
        return $this->heft();
        //            ^ d: none
    }
}

final class Pannier extends Command implements ResetInterface
{
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: none
    }
}

final class Pail extends Command implements Graded, Ranked
{
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: picker src/Storage/Graded.php:7, src/Storage/Ranked.php:7
    }
}

final class Keg extends Cask implements Graded
{
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: src/Storage/Cask.php:7
    }
}
