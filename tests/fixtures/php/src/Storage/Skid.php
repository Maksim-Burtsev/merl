<?php

namespace Shop\Storage;

#[Stacked] abstract class Skid
{
    public function shrink(): int
    {
        return 2;
    }
}
