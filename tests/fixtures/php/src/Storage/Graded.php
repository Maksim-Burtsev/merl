<?php

namespace Shop\Storage;

interface Graded
{
    const TIER = 1;

    public function heft(): int;
}
