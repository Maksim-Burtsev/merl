<?php

namespace Shop\Storage;

use Shop\Storage\Legacy\Tags;

class Label
{
    use Tags;

    public function print(): void
    {
        $this->tag();
        //     ^ d: src/Storage/Legacy/Tags.php:7
        //     status: via $this: Label
    }
}
