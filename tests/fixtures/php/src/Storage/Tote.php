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

final class Satchel implements Graded
{
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: src/Storage/Graded.php:7
    }
}

final class Hamper implements Graded { const TIER = 2;
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: picker src/Storage/Tote.php:59, src/Storage/Graded.php:7, …
    }
}

final class Coffer implements Graded { public const TIER = 2;
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: picker src/Storage/Tote.php:67, src/Storage/Graded.php:7, …
    }
}

#[\Attribute]
final class Valise implements Graded { #[\Deprecated] const TIER = 2;
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: picker src/Storage/Tote.php:76, src/Storage/Graded.php:7, …
    }
}

final class Bushel implements Graded { const LOW = 1; const TIER = 2; }

final class Sack
{
    public function grade(): int
    {
        return Bushel::TIER;
        //             ^ d: src/Storage/Tote.php:84
    }
}

abstract class Trunk extends Command implements Graded
{
}

final class Locker extends Trunk
{
    public function grade(): int
    {
        return parent::TIER;
        //             ^ d: src/Storage/Graded.php:7
    }
}

interface Sorted extends Graded
{
}

final class Pouch extends Command implements Sorted
{
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: src/Storage/Graded.php:7
    }
}

interface Banded extends Graded { const TIER = 5;
}

final class Creel extends Command implements Banded
{
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: none
    }
}

#[\Attribute] final class Satchel2 implements Graded { const TIER = 2;
    public function grade(): int
    {
        return self::TIER;
        //           ^ d: !jump
    }
}
