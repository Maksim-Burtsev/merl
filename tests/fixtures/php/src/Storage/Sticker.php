<?php

namespace Shop\Storage;

class Sticker
{
    use Tags;

    public function print(): void
    {
        $this->tag();
        //     ^ d: src/Storage/Tags.php:7
    }
}
