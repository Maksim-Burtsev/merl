<?php

namespace Shop\Storage;

class Bin extends Shelf
{
    use Stacks;

    public function __construct(private readonly Rack $loader)
    {
    }

    public function load(): void
    {
        $this->stow('a', 'b');
        //     ^ d: src/Storage/Bin.php:45
        //     status: stow → Bin::stow (via $this: Bin)
        $this->loader->seal();
        //     ^ d: src/Storage/Bin.php:9
        //     status: via $this: Bin
        $this->stack();
        //     ^ d: src/Storage/Stacks.php:7
        //     status: via $this: Bin
        $this->lift();
        //     ^ d: src/Storage/Shelf.php:9
        //     status: lift → Shelf::lift (via $this: Bin)
        self::MODE;
        //    ^ d: src/Storage/Shelf.php:7
        //    status: via self: Bin
        static::MODE;
        //      ^ d: src/Storage/Shelf.php:7
        //      status: via static: Bin
        parent::lift();
        //      ^ d: src/Storage/Shelf.php:9
        //      status: via parent of Bin
        $peek = new class() extends Shelf {
            public function peek(): void
            {
                $this->lift();
                //     ^ d: picker src/Storage/Shelf.php:9, src/Storage/Rack.php:14
            }
        };
    }

    private function stow(string $file, string $contents): void
    {
    }
}
