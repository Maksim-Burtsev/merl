<?php

namespace Shop\Radio;

function radio_branding(): Branding
{
    return new Branding();
}

function radio_defaults(?Branding $given): array
{
    return [
        'brand' => radio_branding(),
        //         ^ d: src/Radio/helpers.php:5
        'given' => $given ?: radio_branding(),
        //                   ^ d: src/Radio/helpers.php:5
    ];
}

class RadioDefaults
{
    public function radio_branding(): void
    {
    }
}
