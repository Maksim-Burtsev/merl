<?php

namespace Shop\Radio;

use Illuminate\Support\Facades\File;

class Settings
{
    public function __construct(
        private readonly array $config = [],
    ) {
    }

    public function defaults(): array
    {
        $config = ['streaming' => 'php'];
        $copy = clone $this;
        return [$config, $copy];
    }

    public function load(): int
    {
        $limit = config('radio.limit');
        //       ^ d: none
        File::copy('a', 'b');
        //    ^ d: none
        $this->config;
        //     ^ d: src/Radio/Settings.php:10
        return $limit;
        //      ^ d: src/Radio/Settings.php:23
    }
}
