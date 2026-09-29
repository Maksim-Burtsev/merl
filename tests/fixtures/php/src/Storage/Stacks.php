<?php

namespace Shop\Storage;

trait Stacks
{
    protected function stack(): void
    {
        $this->lift();
        //     ^ d: picker src/Storage/Shelf.php:9, src/Storage/Rack.php:14
    }
}
