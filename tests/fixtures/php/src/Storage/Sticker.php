<?php

namespace Shop\Storage;

class Sticker
{
    use Tags;

    public function print(): void
    {
        $this->tag();
        //     ^ d: picker src/Storage/Legacy/Tags.php:7, src/Storage/Tags.php:7
    }
}
