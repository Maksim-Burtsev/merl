<?php

namespace Shop\Storage;

abstract class Shelf
{
    public const MODE = 'rw';

    protected function lift(): void
    {
    }
}
